#![feature(proc_macro)]

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_aktor;
#[macro_use]
extern crate error_chain;

extern crate uuid;

extern crate aktors;
extern crate channel;
extern crate rustlearn;
extern crate rand;
extern crate stopwatch;
extern crate futures;
extern crate sentiment as _sentiment;
extern crate mailparse;
extern crate walkdir;
extern crate redis;
extern crate twox_hash;
extern crate byteorder;
extern crate lru_time_cache;
extern crate select;

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

    let workers = get_workers(22, system.clone());
    println!("aa");
    loop {
        std::thread::park();
    }
}

fn get_workers(count: usize, system: SystemActor) -> Vec<SpamDetectionServiceActor> {
    let mut workers = Vec::with_capacity(count);

    for _ in 0..count {
        let service = gen_worker(system.clone());
        workers.push(service);
    }

    let mut file_reader_workers = Vec::new();

    for _ in 0..50 {
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

    let file_reader_pool = FileReaderPoolActor::new(file_reader_pool, system.clone(),
                                                    Duration::from_secs(30));


    let w = workers.clone();
    let email_reader = move |self_ref, system|
        EmailReader::new(w.clone().into_iter(),
                         file_reader_pool.clone(),
                         self_ref,
                         system);

    let email_reader = EmailReaderActor::new(email_reader, system.clone(), Duration::from_secs(30));
    email_reader.start("./TRAINING/".to_owned().into());

    for worker in workers.clone() {
        email_reader.request_next_file(worker.clone().id.as_ref().to_owned());
    }

    workers
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

    let model = move |self_ref, system| Model::new(self_ref, system);
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
    fn test_name() {
        let system = SystemActor::new();

        let worker = gen_worker(system);



    }
}