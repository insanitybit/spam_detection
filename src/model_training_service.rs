use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};

use std;
use std::sync::Arc;
use std::hash::Hasher;

use errors::*;
use email::*;
use model::*;
use extraction::*;
use state::*;
use email_reader::*;
use feature_storage::*;

pub struct ModelTrainingService {
    self_ref: ModelTrainingServiceActor,
    system: SystemActor,
    extractor: FeatureExtractionManagerActor,
    feature_storage: FeatureStoreActor
}

#[derive_actor]
impl ModelTrainingService {
    pub fn train_model(&mut self, email: EmailBytes, is_malicious: bool) {
        let self_ref = self.self_ref.clone();
        let feature_storage = self.feature_storage.clone();
        let hash = ModelTrainingService::hash_email(email.clone());

        self.feature_storage.store_truth_value(hash.clone(), is_malicious);

        self.extractor.extract(email, Arc::new(move |features| {
            match features {
                Ok(features) => {
                    let sentiment_features = features.sentiment_analysis;
                    feature_storage.store_sentiment_features(hash.clone(), sentiment_features);
                }
                Err(e) => {}
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

impl ModelTrainingService {
    pub fn new(extractor: FeatureExtractionManagerActor,
               feature_storage: FeatureStoreActor,
               self_ref: ModelTrainingServiceActor,
               system: SystemActor) -> ModelTrainingService {
        ModelTrainingService {
            self_ref,
            system,
            extractor,
            feature_storage,
        }
    }

    fn on_timeout(&mut self) {
        // TODO: Call 'on_error'
    }

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: ModelTrainingServiceMessage,
                   t: Arc<T>)
        where T: Fn(ModelTrainingServiceActor, SystemActor) -> ModelTrainingService + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}