#![feature(proc_macro)]

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_aktor;
#[macro_use]
extern crate error_chain;


extern crate aktors;
extern crate byteorder;
extern crate channel;
extern crate futures;
extern crate lru_time_cache;
extern crate mailparse;
extern crate rand;
extern crate rayon;
extern crate redis;
extern crate reqwest;
extern crate select;
extern crate sentiment as _sentiment;
extern crate stopwatch;
extern crate twox_hash;
extern crate uuid;
extern crate walkdir;

macro_rules! random_panic {
    ($x:expr) => {
        #[cfg(debug_assertions)]
        {
            let mut rng = ::rand::weak_rng();
            let panic = ::rand::Rng::gen_weighted_bool(&mut rng, $x);
            if panic {
                panic!(format!("Unforeseeable error in: {} {}", module_path!(), line!()));
            }
        }
    };
}

macro_rules! random_latency {
    ($x:expr, $y:expr) => {
        #[cfg(debug_assertions)]
        {
            let mut rng = ::rand::weak_rng();
            let lag = ::rand::Rng::gen_weighted_bool(&mut rng, $x);

            if lag {
                ::std::thread::sleep(::std::time::Duration::from_millis($y));
            }
        }
    };
}

pub mod errors;
pub mod sentiment;
pub mod email;
pub mod extraction;
pub mod model;
pub mod service;
pub mod state;
pub mod email_reader;
pub mod html;
pub mod files;

use aktors::actor::SystemActor;
use stopwatch::Stopwatch;
use std::time::Duration;
use walkdir::WalkDir;
use std::collections::HashMap;
use rayon::prelude::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;

use errors::*;
use sentiment::*;
use email::*;
use extraction::*;
use model::*;
use service::*;
use email_reader::*;
use state::*;
use files::*;

fn main() {
    // TODO: A macro where message timings (send/ receive duration) are automatically sent to
    // TODO: some separate place, should just be a matter of having a stopwatch on the sender
    // TODO: and then call it right before route_msg

    let system = SystemActor::new();

    let mut walker = WalkDir::new("./TRAINING/");
    let paths = walker
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|p| p.file_type().is_file())
        .map(|s| s.path().to_owned())
        .filter(|p| p.extension() == Some(std::ffi::OsStr::new("eml")))
        .map(|s| s)
        .collect::<Vec<_>>();

    let worker = get_workers(22, system.clone());

    let mut sw = Stopwatch::new();
    let (tx, rx) = channel::unbounded();
    let mut path_count = 0;
    let mut path_dups = HashMap::new();
    for path in paths.into_iter() {
        path_count += 1;
        *path_dups.entry(path.clone()).or_insert(0) += 1;
        let tx = tx.clone();
        worker.add_file(path.clone(), Arc::new(move |prediction| {
            println!("{:#?} prediction {:#?}", path, prediction);
            tx.send((path.clone(), prediction));
        }));
        std::thread::sleep(Duration::from_millis(2));
    }

    drop(worker);

    for (p, c) in path_dups {
        if c > 1 {
            println!("DUPLICATED PATH {:#?}", p);
        }
    }

    let mut ok_count = HashMap::new();
    let mut aborted = HashMap::new();
    let mut retry_count = HashMap::new();


    sw.start();
    for (path, prediction) in rx {
        if prediction.is_ok() {
            let count = ok_count.entry(path.clone()).or_insert(0);
            *count += 1;
            let count = *count;
            if count > 1 {
                println!("ok_count > 1");
            }
        }

        if let Err(e) = prediction {
            match *e.kind() {
                ErrorKind::RecoverableError(ref e) => {
                    *retry_count.entry(path.clone()).or_insert(0) += 1;
                }
                ErrorKind::UnrecoverableError(ref e) => {
                    let count = aborted.entry(path.clone()).or_insert(0);
                    *count += 1;
                    let count = *count;
                    if count > 1 {
                        println!("aborted > 1");
                    }
                }
                ErrorKind::Msg(ref e) => {
                    *retry_count.entry(path.clone()).or_insert(0) += 1;
                }
                _ => *retry_count.entry(path.clone()).or_insert(0) += 1,
            }
        }

        println!("total count {} ok_count {} aborted {} tried {}",
                 path_count,
                 ok_count.len(),
                 aborted.len(),
                 retry_count.len());
        if ok_count.len() + aborted.len() == path_count {
            break
        }
    }

    println!("{} millis", sw.elapsed_ms());
    //    loop {
    //        std::thread::park();
    //    }
}

fn get_workers(count: usize, system: SystemActor) -> EmailReaderActor {
    let mut workers = Vec::with_capacity(count);

    vec![(); count]
        .par_iter()
        .map(|_| gen_worker(system.clone()))
        .collect_into(&mut workers);

    let file_reader_pool = file_reader_pool(system.clone(), 16);

    let w = workers.clone();
    let email_reader = move |self_ref, system|
        EmailReader::new(w.clone().into_iter(),
                         file_reader_pool.clone(),
                         self_ref,
                         system);

    let email_reader = EmailReaderActor::new(email_reader, system.clone(), Duration::from_secs(30));

    email_reader
}


fn file_reader_pool(system: SystemActor, count: usize) -> FileReaderPoolActor {
    let mut file_reader_workers = Vec::new();

    for _ in 0..count {
        let file_reader =
            move |self_ref, system| LocalFileReader::new(self_ref, system);
        let file_reader = LocalFileReaderActor::new(file_reader, system.clone(),
                                                    Duration::from_secs(30));

        file_reader_workers.push(file_reader);
    }

    println!("{:#?}", file_reader_workers.len());
    let file_reader_pool = move |self_ref, system|
        FileReaderPool::new(
            file_reader_workers.clone().into_iter(),
            self_ref,
            system);

    FileReaderPoolActor::new(file_reader_pool, system.clone(),
                             Duration::from_secs(30))
}

fn gen_worker(system: SystemActor) -> SpamDetectionServiceActor {
    let prediction_cache =
        move |self_ref, system| PredictionCache::new(self_ref, system);
    let prediction_cache = PredictionCacheActor::new(prediction_cache, system.clone(), Duration::from_secs(30));

    let mail_parser =
        move |self_ref, system| MailParser::new(self_ref, system);
    let mail_parser = MailParserActor::new(mail_parser, system.clone(), Duration::from_secs(30));

    let sentiment_analyzer =
        move |self_ref, system| SentimentAnalyzer::new(self_ref, system);
    let sentiment_analyzer = SentimentAnalyzerActor::new(sentiment_analyzer, system.clone(), Duration::from_secs(30));

    let python_model =
        move |self_ref, system| PythonModel::new("./model_service/service/prediction_service.py".into());
    let python_model = PythonModelActor::new(python_model, system.clone(), Duration::from_secs(30));

    let model =
        move |self_ref, system| Model::new(self_ref, system, python_model.clone());
    let model = ModelActor::new(model, system.clone(), Duration::from_secs(30));

    let extractor =
        move |self_ref, system|
            FeatureExtractionManager::new(mail_parser.clone(), sentiment_analyzer.clone(), self_ref, system);
    let extractor = FeatureExtractionManagerActor::new(extractor, system.clone(), Duration::from_secs(30));

    let service =
        move |self_ref, system| SpamDetectionService::new(
            prediction_cache.clone(),
            extractor.clone(),
            model.clone(),
            self_ref,
            system
        );

    SpamDetectionServiceActor::new(service, system.clone(), Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_test() {
        let system = SystemActor::new();

        let worker = gen_worker(system);
    }
}