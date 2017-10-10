#![feature(proc_macro)]

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_aktor;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
#[macro_use]
extern crate error_chain;


extern crate aktors;
extern crate bigdecimal;
extern crate byteorder;
extern crate channel;
extern crate dotenv;
extern crate futures;
extern crate lru_time_cache;
extern crate mailparse;
extern crate postgres;
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

//macro_rules! try_print_panic {
//    ($x:expr) => {
//        {
////            if let Some(m) = $x.downcast_ref::<String>() {
////                println!("It's a string({}): '{}'", string.len(), string);
////            } else if let Some(m) = $x.downcast_ref::<&str>() {
////                println!("It's a string({}): '{}'", string.len(), string);
////            } else {
////                println!("Not a string...");
////            }
//        }
//    };
//}

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
pub mod models;
pub mod spam_detection_service;
pub mod state;
pub mod email_reader;
pub mod html;
pub mod files;
pub mod feature_storage;
pub mod schema;
pub mod model_training_service;

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
use feature_storage::*;
use model::*;
use spam_detection_service::*;
use email_reader::*;
use state::*;
use files::*;
use model_training_service::*;

fn main() {
    // TODO: A macro where message timings (send/ receive duration) are automatically sent to
    // TODO: some separate place, should just be a matter of having a stopwatch on the sender
    // TODO: and then call it right before route_msg

    if false {
        predict();
    } else {
        train();
    }

    std::thread::park();
}

fn train() {
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


    let mail_parser =
    move |self_ref, system| MailParser::new(self_ref, system);
    let mail_parser = MailParserActor::new(mail_parser, system.clone(), Duration::from_secs(30));

    let sentiment_analyzer =
    move |self_ref, system| SentimentAnalyzer::new(self_ref, system);
    let sentiment_analyzer = SentimentAnalyzerActor::new(sentiment_analyzer, system.clone(), Duration::from_secs(30));

    let extractor =
    move |self_ref, system|
    FeatureExtractionManager::new(mail_parser.clone(), sentiment_analyzer.clone(), self_ref, system);
    let extractor = FeatureExtractionManagerActor::new(extractor, system.clone(), Duration::from_secs(30));


    let storage =
    move |self_ref, system|
    FeatureStore::new( self_ref, system);
    let storage = FeatureStoreActor::new(storage, system.clone(), Duration::from_secs(30));

    let trainer =
    move |self_ref, system|
    ModelTrainingService::new(extractor.clone(), storage.clone(), self_ref, system);
    let trainer = ModelTrainingServiceActor::new(trainer, system.clone(), Duration::from_secs(30));


    let truth_values = truth_values();


    for path in paths.into_iter() {
        let filename = path.file_name().expect("filename1");
        let filename = filename.to_str().expect("filename2");

        let truth = truth_values.get(filename).expect("truth");

        let mut email = Vec::new();
        let mut file = File::open(path.clone()).expect("open file");
        file.read_to_end(&mut email);
        trainer.train_model(Arc::new(email), *truth);
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn truth_values() -> HashMap<String, bool> {
    let mut f = File::open("./TRAINING/SPAMTrain.label").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s);

    let lines: Vec<_> = s.split("\n").collect::<Vec<_>>();
    let mut map = HashMap::new();
    for line in lines {
        if line.is_empty() { continue }
        // 0 = spam
        let line: &str = line;
        let mut s = line.split(" ");
        let score = s.next().expect("score next");
        let score = if score == "0" {
            true
        } else if score == "1" {
            false
        } else {
            panic!("bad parse: {} {}", line, score);
        };

        let name = s.next().expect("name next");
        map.insert(name.to_owned(), score);
    }
    map
}

fn predict() {
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
    sw.start();

    let mut r_sw = Stopwatch::new();
    r_sw.start();
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
    r_sw.stop();
    drop(worker);

    for (p, c) in path_dups {
        if c > 1 {
            println!("DUPLICATED PATH {:#?}", p);
        }
    }

    let mut ok_count = HashMap::new();
    let mut aborted = HashMap::new();
    let mut retry_count = HashMap::new();


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

    println!("reading + processing took {} millis", sw.elapsed_ms());
    println!("file reading took {} millis", r_sw.elapsed_ms());
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
        move |self_ref, system| PythonModel::new(self_ref, system, "./model_service/service/prediction_service.py".into());
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