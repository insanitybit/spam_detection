use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use mailparse::*;
use std;
use std::sync::Arc;

use errors::*;

pub struct MailParser {
    self_ref: MailParserActor,
    system: SystemActor
}

pub type EmailBytes = std::sync::Arc<Vec<u8>>;
type ParseResult = std::sync::Arc<Fn(Result<ParsedMail>) + Send + Sync + 'static>;

#[derive_actor]
impl MailParser {
    pub fn parse(&self, data: EmailBytes, res: ParseResult) {
        let mail = parse_mail(&data)
            .map_err(|e|
                ErrorKind::UnrecoverableError(format!("Failed to parse mail with {}", e).into())
                    .into()
            );

        res(mail)
    }
}

impl MailParser {
    pub fn new(self_ref: MailParserActor, system: SystemActor) -> MailParser {
        MailParser {
            self_ref,
            system
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: MailParserMessage,
                   t: Arc<T>)
        where T: Fn(MailParserActor, SystemActor) -> MailParser + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

