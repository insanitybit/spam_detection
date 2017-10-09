use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::sync::Arc;
use std::hash::Hasher;

use errors::*;
use email::*;
use model::*;
use extraction::*;
use state::*;
use email_reader::*;

pub struct ModelTrainingService {
    self_ref: ModelTrainingServiceActor,
    system: SystemActor,
    extractor: FeatureExtractionManagerActor,
}

#[derive_actor]
impl ModelTrainingService {
    pub fn train_model(&mut self, email: EmailBytes) {
        let self_ref = self.self_ref.clone();

        self.extractor.extract(email, std::sync::Arc::new(move |features| {
            match features {
                Ok(data) => {

                }
                Err(e) => {

                }
            };
        }));
    }


}

impl ModelTrainingService {
    pub fn new(prediction_cache: PredictionCacheActor,
               extractor: FeatureExtractionManagerActor,
               model: ModelActor,
               self_ref: ModelTrainingServiceActor,
               system: SystemActor) -> ModelTrainingService {
        ModelTrainingService {
            self_ref,
            system,
            extractor,
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