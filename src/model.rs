use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::time::Duration;
use std::sync::Arc;
use rustlearn::prelude::*;

use rustlearn::ensemble::random_forest::Hyperparameters;
use rustlearn::ensemble::random_forest::RandomForest;
use rustlearn::datasets::iris;
use rustlearn::trees::decision_tree;

use lru_time_cache::LruCache;

use errors::*;
use extraction::Features;

use redis::{self, Connection, Commands};

pub struct Model {
    self_ref: ModelActor,
    system: SystemActor,
    model: RandomForest,
    predictions: usize
}

type Prediction = std::sync::Arc<Fn(Result<bool>) + Send + Sync + 'static>;

#[derive_actor]
impl Model {
    pub fn predict(&mut self, features: Features, res: Prediction) {
        //        let data = Array::from(&vec![vec![features.sentiment_analysis.score,
        //                                          features.sentiment_analysis.positive.score,
        //                                          features.sentiment_analysis.negative.score]]);
        std::thread::sleep(Duration::from_millis(10));
        self.predictions += 1;
        //        println!("predict: {} {}", self.self_ref.id.clone(), self.predictions);

        res(Ok(true));
        //        println!("{:#?}", data);
        //        res(*self.model.predict(&data).expect("Failed to predict").data().iter().next().expect("prediction next unwrap") > 0.5);
    }
}

impl Model {
    pub fn new(self_ref: ModelActor, system: SystemActor) -> Model {
        let mut tree_params = decision_tree::Hyperparameters::new(3);
        tree_params.min_samples_split(10)
            .max_features(4);


        //        let data = Array::from(&vec![vec![0.0, 1.0, 1.0],
        //                                     vec![2.0, 3.0, 3.0]]);
        //
        //        let answers = Array::from(&vec![vec![0.0, 1.0, 1.0],
        //                                        vec![2.0, 3.0, 3.0]]);

        let mut model = Hyperparameters::new(tree_params, 10).build();
        //        model.fit(&data, &answers);

        Model {
            self_ref,
            system,
            model,
            predictions: 0
        }
    }

    pub fn from_rf(model: RandomForest, self_ref: ModelActor, system: SystemActor) -> Model {
        Model {
            self_ref,
            system,
            model,
            predictions: 0
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: ModelMessage,
                   t: Arc<T>)
        where T: Fn(ModelActor, SystemActor) -> Model + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

pub struct PredictionCache {
    self_ref: PredictionCacheActor,
    system: SystemActor,
    cache: LruCache<Vec<u8>, bool>
    //    connection: Connection
}

type GetResponse = std::sync::Arc<Fn(Result<Option<bool>>) + Send + Sync + 'static>;
type Hash = Vec<u8>;

#[derive_actor]
impl PredictionCache {
    pub fn get(&mut self, email_hash: Hash, res: GetResponse) {
        let mut email_hash = email_hash;
        email_hash.extend_from_slice(&b"prediction"[..]);

        random_panic!(10);
        random_latency!(10, 20);

        res(Ok(self.cache.get(&email_hash).cloned()));
    }

    pub fn set(&mut self, email_hash: Hash, prediction: bool) {
        let mut email_hash = email_hash;
        email_hash.extend_from_slice(&b"prediction"[..]);

        self.cache.insert(email_hash, prediction);
    }
}

impl PredictionCache {
    pub fn new(self_ref: PredictionCacheActor, system: SystemActor) -> PredictionCache {
        //        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        //        let con = client.get_connection().unwrap();

        let time_to_live = std::time::Duration::from_secs(60);
        PredictionCache {
            self_ref,
            system,
            cache: LruCache::with_expiry_duration_and_capacity(time_to_live, 10),
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: PredictionCacheMessage,
                   t: Arc<T>)
        where T: Fn(PredictionCacheActor, SystemActor) -> PredictionCache + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}