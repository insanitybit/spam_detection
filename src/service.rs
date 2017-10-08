use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::sync::Arc;
use std::hash::Hasher;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};

use errors::*;
use email::*;
use model::*;
use extraction::*;
use state::*;
use email_reader::*;

pub struct SpamDetectionService {
    self_ref: SpamDetectionServiceActor,
    system: SystemActor,
    prediction_cache: PredictionCacheActor,
    extractor: FeatureExtractionManagerActor,
    model: ModelActor,
}

type PredictionResult = std::sync::Arc<Fn(Result<bool>) + Send + Sync + 'static>;

type PredErr = std::sync::Arc<Error>;

#[derive_actor]
impl SpamDetectionService {
    pub fn predict_with_cache(&mut self, email: EmailBytes, res: PredictionResult) {
        let self_ref = self.self_ref.clone();
        let res = res.clone();
        let email = email.clone();

        let hash = SpamDetectionService::hash_email(email.clone());

        self.prediction_cache
            .get(hash, std::sync::Arc::new(move |cache_res| {
                match cache_res {
                    Ok(Some(hit)) => {
                        println!("pred cache hit");
                        res(Ok(hit));
                    }
                    _ => self_ref.clone().predict(email.clone(), res.clone())
                };
            }));
    }

    pub fn predict(&self, email: EmailBytes, res: PredictionResult) {
        let self_ref = self.self_ref.clone();
        let model = self.model.clone();

        self.extractor.extract(email, std::sync::Arc::new(move |features| {
            match features {
                Ok(data) => {
                    model.clone().predict(data, res.clone())
                }
                Err(e) => {
                    res(Err(e));
                }
            };
        }));
    }

    fn hash_email(email: EmailBytes) -> Vec<u8> {
        let mut hasher = XxHash::default();
        hasher.write(email.as_ref());
        let hash = hasher.finish();
        let mut buf = vec![0; 8];
        LittleEndian::write_u64(&mut buf, hash);
        buf
    }
}

impl SpamDetectionService {
    pub fn new(prediction_cache: PredictionCacheActor,
               extractor: FeatureExtractionManagerActor,
               model: ModelActor,
               self_ref: SpamDetectionServiceActor,
               system: SystemActor) -> SpamDetectionService {
        SpamDetectionService {
            self_ref,
            system,
            prediction_cache,
            extractor,
            model,
        }
    }

    fn on_timeout(&mut self) {
        // TODO: Call 'on_error'
    }

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: SpamDetectionServiceMessage,
                   t: Arc<T>)
        where T: Fn(SpamDetectionServiceActor, SystemActor) -> SpamDetectionService + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}