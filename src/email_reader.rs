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
    file_names: LinkedList<PathBuf>,
    file_reader: FileReaderPoolActor,
}

type ByteVec = Vec<u8>;

pub type FileReadResult = std::sync::Arc<Fn(Arc<Result<EmailBytes>>) + Send + Sync + 'static>;

#[derive_actor]
impl EmailReader
{
    pub fn request_next_file(&mut self, res: FileReadResult) {
        let path = match self.file_names.pop_front() {
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
//        self.file_names.push_back(path.clone());

        let self_ref = self.self_ref.clone();

        self.file_reader.read_file(
            path.clone(),
            res.clone()
        );
    }

    pub fn request_file(&mut self, path: PathBuf, res: FileReadResult) {
        self.fetch_next();
        //        self.file_names.push_back(path.clone());

        let self_ref = self.self_ref.clone();

        self.file_reader.read_file(
            path.clone(),
            res.clone()
        );
    }

    pub fn fetch_next(&mut self) {
        let path = match self.file_names.pop_front() {
            Some(entry) => entry.clone(),
            None => return
        };

        self.file_names.push_back(path.clone());

        self.file_reader.prefetch(path.clone());
    }


    pub fn requeue_file(&mut self, path: PathBuf) {
        self.file_names.push_front(path);
    }

    pub fn load_files(&mut self, path: PathBuf) {
        let mut walker = WalkDir::new(path);
        let paths = walker
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|p| p.file_type().is_file())
            .map(|s| s.path().to_owned())
            .filter_map(|p| {
                if p.extension() == Some(std::ffi::OsStr::new("eml")) {
                    Some(p)
                } else {
                    None
                }
            });

        self.file_names.extend(paths);
    }
}

impl EmailReader
{
    pub fn new(file_reader: FileReaderPoolActor,
                  self_ref: EmailReaderActor,
                  system: SystemActor) -> EmailReader

    {

        EmailReader {
            // As long as we call 'init' before accessing self_ref this
            // is safe
            self_ref,
            system,
            file_names: LinkedList::new(),
            file_reader,
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
