use derive_aktor::derive_actor;

use std;
use std::time::Duration;
use std::sync::{Mutex, Arc};
use _sentiment::Analysis;
use aktors::actor::SystemActor;

use select::document::Document;
use select::predicate::{Predicate, Attr, Class, Name};

use lru_time_cache::LruCache;

use mailparse::*;
use sentiment::*;
use errors::*;
use email::*;

pub struct HtmlParser {
    self_ref: HtmlParserActor,
    system: SystemActor,
}

type ParseResponse = Arc<Fn(Result<Html>) + Send + Sync + 'static>;
type Html = Document;

#[derive_actor]
impl HtmlParser {
    pub fn parse(&mut self, email: String, res: ParseResponse) {
        res(Ok(Document::from(email.as_ref())))
    }
}

impl HtmlParser {
    pub fn new(self_ref: HtmlParserActor, system: SystemActor) -> HtmlParser {
        HtmlParser {
            self_ref,
            system,
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: HtmlParserMessage,
                   t: Arc<T>)
        where T: Fn(HtmlParserActor, SystemActor) -> HtmlParser + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}