use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;

use std::hash::Hasher;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};

use errors::*;
use email::*;
use model::*;
use extraction::*;

use std::sync::Arc;

type CloneableError = Arc<ErrorKind>;

pub struct CompletionHandler<F>
    where F: Fn(CompletionStatus) + Send + Sync + 'static + Send
{
    self_ref: CompletionHandlerActor,
    system: SystemActor,
    f: F,
    tries: usize
}

#[derive(Debug, Clone)]
pub enum CompletionStatus {
    /// Processed successfully
    Success,
    /// A transient error occurred
    Retry(CloneableError, usize),
    /// An unrecoverable Error occurred
    Abort(CloneableError)
}

#[derive_actor]
impl<F> CompletionHandler<F>
    where F: Fn(CompletionStatus) + Send + Sync + 'static
{
    pub fn success(&self) {
        (self.f)(CompletionStatus::Success);
    }

    pub fn retry(&self, e: CloneableError) {
        (self.f)(CompletionStatus::Retry(e, self.tries + 1));
    }

    pub fn abort(&self, e: CloneableError) {
        (self.f)(CompletionStatus::Abort(e));
    }
}

impl<F> CompletionHandler<F>
    where F: Fn(CompletionStatus) + Send + Sync + 'static
{
    pub fn new(tries: usize, f: F, self_ref: CompletionHandlerActor, system: SystemActor) -> CompletionHandler<F> {
        CompletionHandler {
            self_ref,
            system,
            f,
            tries
        }
    }

    fn on_timeout(&mut self) {
        (self.f)(CompletionStatus::Retry(
            Arc::new(
                ErrorKind::RecoverableError(
                    "Failed to call CompletionHandler within timeout".into()
                )
            ),
            self.tries + 1))
    }

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: CompletionHandlerMessage,
                   t: Arc<T>)
        where T: Fn(CompletionHandlerActor, SystemActor) -> CompletionHandler<F> + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }
}

//impl