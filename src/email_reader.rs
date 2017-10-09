use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use lru_time_cache::LruCache;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};
use std::fs::File;
use std::io::prelude::*;
use std::time::Duration;
use std::iter::FromIterator;

use errors::*;
use email::*;
use model::*;
use extraction::*;
use service::*;
use state::*;
use files::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::{WalkDir, DirEntry};

use std::collections::LinkedList;

pub struct EmailReader
{
    self_ref: EmailReaderActor,
    system: SystemActor,
    file_names: LinkedList<(PathBuf, usize, PredictionResult)>,
    workers: HashMap<String, SpamDetectionServiceActor>,
    available_workers: HashMap<String, SpamDetectionServiceActor>,
    file_reader: FileReaderPoolActor,
}

type ByteVec = Vec<u8>;

#[derive_actor]
impl EmailReader
{
    pub fn request_next_file(&mut self, id: String) {
        let (path, tries, res) = match self.file_names.pop_front() {
            Some(p) => {
                p
            }
            None => {
                println!("No more files");
                // If there's no more files, this worker has no work
                let worker = match self.workers.get(&id) {
                    Some(worker) => {self.available_workers.insert(id.clone(), worker.clone());},
                    None => {
                        println!("Failed to get worker with id: {}", id);
                    }
                };

                return;
            }
        };

        self.fetch_next();

        self.available_workers.remove(&id);

        let self_ref = self.self_ref.clone();

        let completion_handler =
            self.gen_completion_handler(id.clone(),
                                        path.clone(),
                                        res.clone(),
                                        tries);

        let res = res.clone();
        self.file_reader.read_file(
            path.clone(),
            Arc::new(move |buf| {
                self_ref.send_work_by_id(buf,
                                         id.clone(),
                                         res.clone(),
                                         completion_handler.clone());
            })
        );
    }

    pub fn fetch_next(&mut self) {
        let (path, tries, res) = match self.file_names.pop_front() {
            Some(entry) => entry.clone(),
            None => return
        };

        self.file_reader.prefetch(path.clone());
        self.file_names.push_back((path.clone(), tries, res));
    }


    pub fn retry_file_by_path(&mut self, id: String, path: PathBuf, res: PredictionResult, tries: usize) {
        println!("retrying {:#?} tries {}", path, tries);
        self.file_names.push_back((path, tries, res));

        self.request_next_file(id);
    }

    fn gen_completion_handler(&self,
                              id: String,
                              path: PathBuf,
                              res: PredictionResult,
                              tries: usize) -> CompletionHandlerActor {
        let self_ref = self.self_ref.clone();

        let c_handler = move |completion_handler, system|
            {
                let self_ref = self_ref.clone();
                let id = id.clone();
                let path = path.clone();
                let res = res.clone();

                CompletionHandler::new(tries,
                                       move |status| {
                                           match status {
                                               CompletionStatus::Success => {
                                                   self_ref.request_next_file(id.clone());
                                               }
                                               CompletionStatus::Abort(e) => {
                                                   self_ref.request_next_file(id.clone());
                                               }
                                               CompletionStatus::Retry(e, tries) => {
                                                   std::thread::sleep(Duration::from_millis(2 << tries as u64));
                                                   if tries > 5 {
                                                       self_ref.request_next_file(id.clone());
                                                   } else {
                                                       self_ref.retry_file_by_path(id.clone(), path.clone(), res.clone(), tries);
                                                   }
                                               }
                                           }
                                       },
                                       completion_handler,
                                       system)
            };

        return CompletionHandlerActor::new(c_handler, self.system.clone(), Duration::from_secs(30));
    }

    pub fn send_work_by_id(&self, work: EmailBytes, id: String, res: PredictionResult, completion_handler: CompletionHandlerActor) {
        let worker = match self.workers.get(&id) {
            Some(worker) => worker,
            None => {
                println!("Failed to get worker with id: {}", id);
                return;
            }
        };

        let worker_ref = worker.clone();

        let self_ref = self.self_ref.clone();

        let res = res.clone();
        worker.predict(work, Arc::new(move |p| {
            match p {
                Ok(p) => {
                    completion_handler.success();
                }
                Err(ref e) => {
                    match *e.kind() {
                        ErrorKind::RecoverableError(ref e) => {
                            completion_handler
                                .retry(Arc::new(ErrorKind::RecoverableError(e.to_owned().into())));
                        }
                        ErrorKind::UnrecoverableError(ref e) => {
                            completion_handler
                                .abort(Arc::new(ErrorKind::UnrecoverableError(e.to_owned().into())));
                        }
                        ErrorKind::Msg(ref e) => {
                            completion_handler
                                .retry(Arc::new(e.as_str().into()));
                        }
                        _ => {
                            completion_handler
                                .retry(Arc::new("An unknown error occurred".into()));
                        }
                    }
                }
            };
            res(p);
        })
        );
    }

    pub fn add_file(&mut self, path: PathBuf, res: PredictionResult) {
        // Add the file to our queue
        self.file_names.push_back((path, 0, res));

        // If we have a worker who's ready, send it the work directly
        let k = match self.available_workers.keys().next() {
            Some(k) => k.to_owned(),
            None => return
        };

        let worker = match self.available_workers.remove(&k) {
            Some(w) => w.id.as_ref().to_owned(),
            None => return
        };

        // If a worker is available immediately schedule it
        self.request_next_file(worker);
    }
}

impl EmailReader
{
    pub fn new<T>(workers: T,
                  file_reader: FileReaderPoolActor,
                  self_ref: EmailReaderActor,
                  system: SystemActor) -> EmailReader
        where T: Iterator<Item=SpamDetectionServiceActor>
    {
        let mut worker_map = HashMap::new();

        for worker in workers {
            let id = worker.clone().id.as_ref().to_owned();
            worker_map.insert(id.clone(), worker.clone());
        }

        EmailReader {
            // As long as we call 'init' before accessing self_ref this
            // is safe
            self_ref,
            system,
            file_names: LinkedList::new(),
            file_reader,
            workers: worker_map.clone(),
            available_workers: HashMap::from(worker_map)
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: EmailReaderMessage,
                   t: Arc<T>)
        where T: Fn(EmailReaderActor, SystemActor) -> EmailReader + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}
