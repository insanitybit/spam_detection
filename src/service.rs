use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::sync::Arc;
use std::time::Duration;
use std::hash::Hasher;
use std::path::PathBuf;
use twox_hash::XxHash;
use lru_time_cache::LruCache;
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
    email_reader: EmailReaderActor,
    tries: LruCache<PathBuf, usize>
}

type PredictionResult = std::sync::Arc<Fn(Result<bool>) + Send + Sync + 'static>;

type PredErr = std::sync::Arc<Error>;

#[derive_actor]
impl SpamDetectionService {
    pub fn predict_with_cache(&mut self, path: PathBuf, res: PredictionResult) {
        let completion_handler = self.gen_completion_handler(path.clone(), res.clone(), 0);

        let _completion_handler = completion_handler.clone();
        let new_res = Arc::new(move |p: Result<bool>| {
            match p {
                Ok(p) => {
                    completion_handler.success();
                }
                Err(ref e) => {
                    match *e.kind() {
                        ErrorKind::RecoverableError(ref e) => {
                            completion_handler.retry(Arc::new(ErrorKind::RecoverableError(e.to_owned().into())));
                        },
                        ErrorKind::UnrecoverableError(ref e) =>{
                            completion_handler.abort(Arc::new(ErrorKind::UnrecoverableError(e.to_owned().into())));
                        },
                        ErrorKind::Msg(ref e) => {
                            completion_handler.retry(Arc::new(e.as_str().into()));
                        },
                        _ => {
                            completion_handler.retry(Arc::new("An unknown error occurred".into()));
                        }
                    }
                }
            };

            res(p);
        });


        self.issue_work(path, _completion_handler, new_res);
    }

    pub fn retry_prediction(&mut self, path: PathBuf, res: PredictionResult) {
        let tries = {
            let mut tries = self.tries.entry(path.clone()).or_insert(0);
            *tries += 1;
            *tries
        };

        if tries > 5 {
            res(Err(format!("Repeatedly failed to predict for path {:#?}", path).into()).into());
            return;
        }

        let completion_handler = self.gen_completion_handler(path.clone(), res.clone(), tries);
        self.issue_work(path, completion_handler, res);
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

    fn issue_work(&self, path: PathBuf, completion_handler: CompletionHandlerActor, res: PredictionResult) {
        let self_ref = self.self_ref.clone();
        let prediction_cache = self.prediction_cache.clone();
        self.email_reader.request_file(path, Arc::new(move |email: Arc<Result<EmailBytes>>| {
            let email: EmailBytes = match *email.as_ref() {
                Ok(ref email) => email.clone(),
                Err(_) => {
                    res(Err("Failed to read file".into()));
                    return;
                }
            };

            let hash = SpamDetectionService::hash_email(email.clone());
            let self_ref = self_ref.clone();
            let res = res.clone();
            prediction_cache
                .get(hash, std::sync::Arc::new(move |cache_res| {
                    match cache_res {
                        Ok(Some(hit)) => {
                            println!("pred cache hit");
                            res(Ok(hit));
                        }
                        _ => self_ref.clone().predict(email.clone(), res.clone())
                    };
                }));
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

    fn gen_completion_handler(&self,
                              path: PathBuf,
                              res: PredictionResult,
                              tries: usize) -> CompletionHandlerActor {
        let self_ref = self.self_ref.clone();

        let c_handler = move |completion_handler, system|
            {
                let self_ref = self_ref.clone();
                let path = path.clone();
                let res = res.clone();
                CompletionHandler::new(move |status| {
                    match status {
                        CompletionStatus::Success | CompletionStatus::Abort(_) => {}
                        CompletionStatus::Retry(err) => {
                            // Wait before we request more work
                            //                                                   println!("{} tries", tries);
//                            std::thread::sleep(Duration::from_millis(2 << tries as u64));
                            self_ref.retry_prediction(path.clone(), res.clone());
                        }
                    }
                },
                                       completion_handler,
                                       system)
            };

        return CompletionHandlerActor::new(c_handler, self.system.clone(), Duration::from_secs(30));
    }
}

impl SpamDetectionService {
    pub fn new(prediction_cache: PredictionCacheActor,
               extractor: FeatureExtractionManagerActor,
               model: ModelActor,
               email_reader: EmailReaderActor,
               self_ref: SpamDetectionServiceActor,
               system: SystemActor) -> SpamDetectionService {
        SpamDetectionService {
            self_ref,
            system,
            prediction_cache,
            extractor,
            model,
            email_reader,
            tries: LruCache::with_expiry_duration_and_capacity(Duration::from_secs(120), 10_000)
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