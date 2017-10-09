use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::path::PathBuf;
use std::time::Duration;
use std::sync::Arc;

use lru_time_cache::LruCache;

use errors::*;
use extraction::Features;

use rand::Rng;
use redis::{self, Connection, Commands};
use reqwest::Client;

pub struct Model {
    self_ref: ModelActor,
    system: SystemActor,
    python_model: PythonModelActor,
    predictions: usize
}

type Prediction = std::sync::Arc<Fn(Result<bool>) + Send + Sync + 'static>;

#[derive_actor]
impl Model {
    pub fn predict(&mut self, features: Features, res: Prediction) {
        std::thread::sleep(Duration::from_millis(10));

        self.python_model.predict(features, res);
    }
}

impl Model {
    pub fn new(self_ref: ModelActor,
               system: SystemActor,
               python_model: PythonModelActor) -> Model {
        Model {
            self_ref,
            system,
            python_model,
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

use std::process::{Child, Command};


pub struct PythonModel {
    self_ref: PythonModelActor,
    system: SystemActor,
    python: Child,
    client: Client,
    port: u16,
    path: PathBuf
}

#[derive_actor]
impl PythonModel {
    pub fn predict(&mut self, features: Features, res: Prediction) {
        let query = format!("{},{},{}",
                            features.sentiment_analysis.positive_sentiment,
                            features.sentiment_analysis.negative_sentiment,
                            features.sentiment_analysis.sentiment_score);

        let mut sw = ::stopwatch::Stopwatch::new();
        sw.start();
        let r = self.client.get(&format!("http://127.0.0.1:{}/predict/{}", self.port, query)).send();

        match r {
            Err(e) => {
                res(Err(ErrorKind::RecoverableError(
                    format!("Failed to predict {}", e).into())
                    .into()));
            }
            Ok(mut r) => {
//                println!("SERVER RESPONSE {:#?}", r);
                use std::io::prelude::*;
                let mut content = String::with_capacity(4);
                r.read_to_string(&mut content);
                let pred = if content == "T" {
                    Ok(true)
                } else if content == "F" {
                    Ok(false)
                } else {
                    Err("Failed to parse python service response as boolean".into())
                };

                res(pred);
            }
        }
        println!("flask request {}ms", sw.elapsed_ms());
    }
}

impl PythonModel {
    pub fn new(self_ref: PythonModelActor,
               system: SystemActor,
               path: PathBuf) -> PythonModel {
        let mut rng = ::rand::weak_rng();
        let port: u16 = rng.gen_range(10000, 16000);
        let python =
            Command::new(path.to_str().unwrap())
                .env("FLASK_APP", path.to_str().unwrap())
                .arg("--port")
                .arg(port.to_string())
                .spawn()
                .expect(&format!("Invalid path: {:#?}", path));

        let client = Client::new();
        let url = format!("http://127.0.0.1:{}/health_check", port.to_string());
        let mut up = false;
        for i in 0..15 {
            std::thread::sleep(Duration::from_millis(2 << i));
            if client.get(&url)
                .send()
                .is_ok() {
                println!("Connected to PythonModel");
                up = true;
                break;
            } else {
                println!("Not connected to PythonModel")
            }
        }

        if !up {
            panic!("Could not connect to Python service");
        }

        PythonModel {
            self_ref,
            system,
            python,
            client,
            port,
            path
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: PythonModelMessage,
                   t: Arc<T>)
        where T: Fn(PythonModelActor, SystemActor) -> PythonModel + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

impl Drop for PythonModel {
    fn drop(&mut self) {
        self.python.kill();
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
