use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::collections::HashMap;
use std::hash::Hasher;
use lru_time_cache::LruCache;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};
use std::fs::File;
use std::io::prelude::*;
use std::time::Duration;

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
    file_names: LinkedList<(PathBuf, usize)>,
    workers: HashMap<String, SpamDetectionServiceActor>,
    file_reader: FileReaderPoolActor,
}

type ByteVec = Vec<u8>;

#[derive_actor]
impl EmailReader
{
    pub fn request_next_file(&mut self, id: String) {
        let (path, tries) = match self.file_names.pop_front() {
            Some(p) => {
                p
            }
            None => {
                println!("No more files");
                return;
            }
        };

        println!("fetched");
        self.fetch_next();
        self.file_names.push_back((path.clone(), tries));

        let self_ref = self.self_ref.clone();

        let completion_handler = self.gen_completion_handler(id.clone(), path.clone(), tries);

        self.file_reader.read_file(
            path.clone(),
            Arc::new(move |buf| {
                self_ref.send_work_by_id(buf, id.clone(), completion_handler.clone());
            })
        );
    }

    pub fn fetch_next(&mut self) {
        let (path, tries) = match self.file_names.pop_front() {
            Some(entry) => entry.clone(),
            None => return
        };

        self.file_names.push_back((path.clone(), tries));

        self.file_reader.prefetch(path.clone());
    }


    pub fn retry_file_by_path(&mut self, id: String, path: PathBuf, tries: usize) {
        println!("retrying {:#?} tries {}", path, tries);
        self.file_names.push_back((path, tries));

        self.request_next_file(id);
    }

    fn gen_completion_handler(&self,
                              id: String,
                              path: PathBuf,
                              tries: usize) -> CompletionHandlerActor {
        let self_ref = self.self_ref.clone();

        let c_handler = move |completion_handler, system|
            {
                let self_ref = self_ref.clone();
                let id = id.clone();
                let path = path.clone();
                CompletionHandler::new(tries,
                                       move |status| {
                                           match status {
                                               CompletionStatus::Success | CompletionStatus::Abort => {
                                                   self_ref.request_next_file(id.clone());
                                               }
                                               CompletionStatus::Retry(tries) => {
                                                   // Wait before we request more work
                                                   println!("{} tries", tries);
                                                   std::thread::sleep(Duration::from_millis(2 << tries as u64));
                                                   if tries > 5 {
                                                       println!("aborting after {} tries", tries);
                                                       self_ref.request_next_file(id.clone());
                                                   } else {
                                                       //                                                       println!("{} tries", tries);
                                                       self_ref.retry_file_by_path(id.clone(), path.clone(), tries);
                                                   }
                                               }
                                           }
                                       },
                                       completion_handler,
                                       system)
            };

        return CompletionHandlerActor::new(c_handler, self.system.clone(), Duration::from_secs(30));
    }

    pub fn send_work_by_id(&self, work: EmailBytes, id: String, completion_handler: CompletionHandlerActor) {
        let worker = match self.workers.get(&id) {
            Some(worker) => worker,
            None => {
                println!("Failed to get worker with id: {}", id);
                return;
            }
        };

        let worker_ref = worker.clone();

        let self_ref = self.self_ref.clone();

        worker.predict(work,
                       Arc::new(move |p| {
                           match p {
                               Ok(p) => {
                                   println!("{}", p);
                                   completion_handler.success();
                               }
                               Err(e) => {
                                   match *e.kind() {
                                       ErrorKind::RecoverableError(ref f) => {
                                           completion_handler.retry();
                                       }
                                       ErrorKind::UnrecoverableError(_) =>
                                           completion_handler.abort(),
                                       ref unknown_error => {
                                           println!("UnknownError Occurred: {:#?}", unknown_error);
                                           completion_handler.retry();
                                       }
                                   }
                               }
                           }
                       }));
    }

    pub fn start(&mut self, path: PathBuf) {
        let mut walker = WalkDir::new(path);
        let paths = walker
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|p| p.file_type().is_file())
            .map(|s| s.path().to_owned())
            .filter(|p| p.extension() == Some(std::ffi::OsStr::new("eml")))
            .map(|s| (s, 0));

        self.file_names.extend(paths);
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
            workers: worker_map,
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
