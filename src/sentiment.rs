use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::sync::Arc;
use _sentiment::*;

use errors::*;

pub struct SentimentAnalyzer {
    self_ref: SentimentAnalyzerActor,
    system: SystemActor
}

// TODO: Make Analysis a result
type SentimentResponse = std::sync::Arc<Fn(Result<Analysis>) + Send + Sync + 'static>;

#[derive_actor]
impl SentimentAnalyzer {
    pub fn analyze(&self, phrase: String, res: SentimentResponse) {

        random_panic!(10);
        random_latency!(10, 20);
        let analysis = analyze(phrase);

        res(Ok(analysis));
    }
}

impl SentimentAnalyzer {
    pub fn new(self_ref: SentimentAnalyzerActor, system: SystemActor) -> SentimentAnalyzer {
        SentimentAnalyzer {
            self_ref,
            system
        }
    }

    fn on_timeout(&mut self) {

    }

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: SentimentAnalyzerMessage,
                   t: Arc<T>)
        where T: Fn(SentimentAnalyzerActor, SystemActor) -> SentimentAnalyzer + Send + Sync + 'static
    {
        match msg {
            SentimentAnalyzerMessage::AnalyzeVariant{
                phrase, res
            } => {
                res(Err(
                    ErrorKind::UnrecoverableError(
                        "An unexpected error occurred in sentiment analyzer".into(

                        )).into())
                );
            },
        };
    }
}