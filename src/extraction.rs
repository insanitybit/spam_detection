use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::time::Duration;
use std::sync::Arc;

use mailparse::*;
use sentiment::*;
use errors::*;
use email::*;
use html::*;

#[derive(Clone)]
#[derive(Builder)]
#[builder(setter(into))]
pub struct Features {
    pub sentiment_analysis: SentimentFeatures,
    //    pub body_length: usize
}


pub struct FeatureExtractionManager {
    self_ref: FeatureExtractionManagerActor,
    system: SystemActor,
    parser: MailParserActor,
    sentiment_analyzer: SentimentAnalyzerActor,
}

#[derive_actor]
impl FeatureExtractionManager {
    pub fn extract(&self, email: EmailBytes, res: FeatureExtraction) {
        let r = res.clone();
        let parser = self.parser.clone();
        let sentiment_analyzer = self.sentiment_analyzer.clone();

        let extractor = move |self_ref, system| {
            let r = r.clone();
            FeatureExtractor::new(
                parser.clone(),
                sentiment_analyzer.clone(),
                move || {
                    r(Err(ErrorKind::RecoverableError("FeatureExtractor timed out".into()).into()));
                },
                self_ref,
                system)
        };

        let extractor = FeatureExtractorActor::new(extractor, self.system.clone(), Duration::from_millis(50));

        extractor.extract(email, res);
    }
}

impl FeatureExtractionManager {
    pub fn new(parser: MailParserActor,
               sentiment_analyzer: SentimentAnalyzerActor,
               self_ref: FeatureExtractionManagerActor,
               system: SystemActor) -> FeatureExtractionManager {
        FeatureExtractionManager {
            // As long as we call 'init' before accessing self_ref this
            // is safe
            self_ref,
            system,
            parser,
            sentiment_analyzer,
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: FeatureExtractionManagerMessage,
                   t: Arc<T>)
        where T: Fn(FeatureExtractionManagerActor, SystemActor) -> FeatureExtractionManager + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

type FeatureExtraction = std::sync::Arc<Fn(Result<Features>) + Send + Sync + 'static>;

pub struct FeatureExtractor<T>
    where T: Fn()
{
    self_ref: FeatureExtractorActor,
    system: SystemActor,
    features: FeaturesBuilder,
    parser: MailParserActor,
    sentiment_analyzer: SentimentAnalyzerActor,
    on_timeout: T,
    timed_out: bool
}


#[derive_actor]
impl<T> FeatureExtractor<T>
    where T: Fn() + Send + Sync + 'static
{
    pub fn extract(&self, email: EmailBytes, res: FeatureExtraction) {
        let self_ref = self.self_ref.clone();
        let sentiment_analyzer = self.sentiment_analyzer.clone();
        let system = self.system.clone();

        self.parser.parse(email, std::sync::Arc::new(move |r| {
            let email = match r {
                Ok(email) => email,
                Err(e) => return res(Err(e))
            };

            let self_ref = self_ref.clone();
            let res = res.clone();

            sentiment_analyzer.analyze(email.get_body().unwrap_or("".to_owned()), std::sync::Arc::new(
                move |analysis| {
                    match analysis {
                        Ok(analysis) => {
                            self_ref.clone().set_sentiment(analysis, res.clone())
                        },
                        Err(e) => {
                            res(Err(e))
                        }
                    }
                }
            ));

            //            let html = HtmlParser::new();
            //            let html = HtmlParserActor::new(html, system.clone(), Duration::from_secs(30));
            //
            //            html.parse(email.get_body().unwrap_or("".to_owned()), std::sync::Arc::new(
            //                move |href| {
            ////                println!("")
            //            }));
        }));
    }

    pub fn set_sentiment(&mut self,
                         analysis: SentimentFeatures,
                         res: FeatureExtraction) {
        self.features.sentiment_analysis(analysis);

        // If we've already timed out, don't bother sending features to the rest of the system
        if self.is_complete() && !self.timed_out {
//            random_panic!(10);
//            random_latency!(10, 20);

            res(Ok(self.features.build().expect("set_sentiment")))
        }
    }

    //    pub fn set_body_length(&mut self,
    //                           body_len: usize,
    //                           res: FeatureExtraction) {
    //        self.features.body_length(body_len);
    //
    //        if self.is_complete() {
    //            res(Ok(self.features.build().expect("set_body_len")))
    //        }
    //    }
}

impl<T> FeatureExtractor<T>
    where T: Fn() + Send + Sync + 'static
{
    pub fn new(parser: MailParserActor,
               sentiment_analyzer: SentimentAnalyzerActor,
               on_timeout: T,
               self_ref: FeatureExtractorActor,
               system: SystemActor) -> FeatureExtractor<T> {
        FeatureExtractor {
            self_ref,
            system,
            features: FeaturesBuilder::default(),
            parser,
            sentiment_analyzer,
            on_timeout,
            timed_out: false
        }
    }


    fn on_timeout(&mut self) {
        if !self.timed_out {
            self.timed_out = true;
            (self.on_timeout)();
            self.self_ref.kill();
        } else {
            println!("already timed out")
        }
    }

    fn on_error<F>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: FeatureExtractorMessage,
                   t: Arc<F>)
        where F: Fn(FeatureExtractorActor, SystemActor) -> FeatureExtractor<T> + Send + Sync + 'static
    {
        self.route_msg(msg);
    }

    fn is_complete(&self) -> bool {
        self.features.build().is_ok()
    }
}

