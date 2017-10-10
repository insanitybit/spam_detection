use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use twox_hash::XxHash;

use byteorder::{ByteOrder, LittleEndian};
use derive_aktor::derive_actor;
use aktors::actor::SystemActor;

use std;
use std::sync::Arc;
use std::hash::Hasher;

use errors::*;
use email::*;
use extraction::*;
use model::*;
use schema::*;
use state::*;
use models::*;
use sentiment::SentimentFeatures;
use email_reader::*;

pub struct FeatureStore {
    self_ref: FeatureStoreActor,
    system: SystemActor,
    db: PgConnection
}

type Hash = Vec<u8>;

#[derive_actor]
impl FeatureStore {
    pub fn store_sentiment_features(&self, key: Hash, features: SentimentFeatures) {
        println!("storing sentiment features");

        let stored_features = NewSentimentFeatures {
            positive_sentiment: features.positive_sentiment.into(),
            negative_sentiment: features.negative_sentiment.into(),
            sentiment_score: features.sentiment_score.into(),
            hash: key
        };

        ::diesel::insert(&stored_features).into(sentiment_features::table)
            .execute(&self.db)
            .expect("Error saving new sentiment features");
    }

    pub fn store_truth_value(&self, key: Hash, truth: bool) {
        println!("storing truth value");

        let truth_value = NewTruthValue {
            truth,
            hash: key
        };

        ::diesel::insert(&truth_value).into(truth_values::table)
            .execute(&self.db)
            .expect("Error saving new truth value");
    }
}

impl FeatureStore {
    pub fn new(self_ref: FeatureStoreActor,
               system: SystemActor) -> FeatureStore {
        FeatureStore {
            self_ref,
            system,
            db: FeatureStore::establish_connection()
        }
    }

    fn on_timeout(&mut self) {
        // TODO: Call 'on_error'
    }

    fn on_error<T>(&mut self,
                   err: Box<std::any::Any + Send>,
                   msg: FeatureStoreMessage,
                   t: Arc<T>)
        where T: Fn(FeatureStoreActor, SystemActor) -> FeatureStore + Send + Sync + 'static
    {
        // t(self.self_ref.clone(), self.system.clone());
    }

    pub fn establish_connection() -> PgConnection {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url)
            .expect(&format!("Error connecting to {}", database_url))
    }
}