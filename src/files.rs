use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::iter::Cycle;
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
use spam_detection_service::*;
use state::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::{WalkDir, DirEntry};

use std::collections::LinkedList;

type FileResponse = std::sync::Arc<Fn(Arc<Vec<u8>>) + Send + Sync + 'static>;

pub struct FileReaderPool<T>
    where T: Iterator<Item=LocalFileReaderActor> + Clone + Send + Sync + 'static
{
    self_ref: FileReaderPoolActor,
    system: SystemActor,
    workers: Cycle<T>,
    cache: LruCache<PathBuf, Arc<Vec<u8>>>
}

#[derive_actor]
impl<T> FileReaderPool<T>
    where T: Iterator<Item=LocalFileReaderActor> + Clone + Send + Sync + 'static
{
    pub fn read_file(&mut self, path: PathBuf, res: FileResponse) {
        let self_ref = self.self_ref.clone();

        let worker = self.workers.next().expect("No file reader worker available");
        if let Some(file) = self.cache.get(&path) {
            println!("Cache hit");
            res(file.clone());
        } else {
            println!("Cache miss");
            worker.read_file(
                path.clone(),
                Arc::new(move |file| {
                    self_ref.cache_file(path.clone(), file.clone());
                    res(file);
                })
            );
        }
    }

    pub fn cache_file(&mut self, path: PathBuf, email: EmailBytes) {
        self.cache.insert(path, email);
    }

    pub fn prefetch(&mut self, path: PathBuf) {
        let self_ref = self.self_ref.clone();

        let worker = self.workers.next().expect("No file reader worker available");

        worker.read_file(
            path.clone(),
            Arc::new(move |file| {
//                random_panic!(10);
//                random_latency!(10, 20);
                self_ref.cache_file(path.clone(), file.clone());
            })
        );
    }
}

impl<T> FileReaderPool<T>
    where T: Iterator<Item=LocalFileReaderActor> + Clone + Send + Sync + 'static
{
    pub fn new(workers: T,
               self_ref: FileReaderPoolActor,
               system: SystemActor) -> FileReaderPool<T>
    {
        FileReaderPool {
            self_ref,
            system,
            workers: workers.cycle(),
            cache: LruCache::with_expiry_duration_and_capacity(Duration::from_secs(120), 5000)
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<F>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: FileReaderPoolMessage,
                   t: Arc<F>)
        where F: Fn(FileReaderPoolActor, SystemActor) -> FileReaderPool<T> + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

pub struct LocalFileReader
{
    self_ref: LocalFileReaderActor,
    system: SystemActor,
}

#[derive_actor]
impl LocalFileReader
{
    pub fn read_file(&mut self, path: PathBuf, res: FileResponse) {
        let mut file = File::open(&path)
            .expect(&format!("Could not open file at path {:#?}", path));
        let mut buf = Vec::with_capacity(10 * 1024);
        file.read_to_end(&mut buf).unwrap();
        res(Arc::new(buf));
    }
}

impl LocalFileReader
{
    pub fn new(self_ref: LocalFileReaderActor, system: SystemActor) -> LocalFileReader
    {
        LocalFileReader {
            self_ref,
            system,
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: LocalFileReaderMessage,
                   t: Arc<T>)
        where T: Fn(LocalFileReaderActor, SystemActor) -> LocalFileReader + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}



