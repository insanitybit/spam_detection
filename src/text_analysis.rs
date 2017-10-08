use derive_aktor::derive_actor;

use std;

use std::hash::Hasher;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};

use errors::*;
use email::*;
use model::*;
use extraction::*;

use std::sync::Arc;

pub struct TextFeatureExtractor
{
    self_ref: CompletionHandlerActor,
    system: SystemActor
}

type Completion = std::sync::Arc<Fn() + Send + Sync + 'static>;

pub struct TextFeatures {
    body_length: usize,
    subject_length: usize,
    body_subject_ration: f32,
    header_count: usize,
    min_header_size: usize,
    max_header_size: usize,
    avg_header_size: usize,
    mimetype_commonality: f32,
    charset_commonality: f32,
    charset_label: f32,
    content_name_commonality: f32,
    unknown_words: f32,
    known_unknown_words_ratio: f32,
    grammar_errors: f32,
    caps_lower_ratio: f32,
    tf_idf: f32
}

#[derive_actor]
impl TextFeatureExtractor
{
    pub fn analyze(&self) {}
}

impl TextFeatureExtractor
{
    pub fn new(f: F, self_ref: CompletionHandlerActor, system: SystemActor) -> TextFeatureExtractor {
        TextFeatureExtractor {
            self_ref,
            system
        }
    }

    fn on_timeout(&mut self) {}

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: TextFeatureExtractorMessage,
                   t: Arc<T>)
        where T: Fn(TextFeatureExtractorActor, SystemActor) -> TextFeatureExtractor + Send + Sync + 'static
    {

        match TextFeatureExtractorMessage {
            TextFeatureExtractorMessage::Kill => return,
            _ => ()
        };
        // t(self.self_ref.clone(), self.system.clone());
    }
}

