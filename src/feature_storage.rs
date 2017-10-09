use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

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
use sentiment::SentimentFeatures;
use email_reader::*;

pub mod models {
    #[derive(Queryable)]
    pub struct StoredSentimentFeatures {
        pub id: i32,
        pub positive_sentiment: f32,
        pub negative_sentiment: f32,
        pub sentiment_score: f32
    }

    #[derive(Insertable)]
    #[table_name = "sentiment_features"]
    pub struct NewSentimentFeatures {
        pub positive_sentiment: f32,
        pub negative_sentiment: f32,
        pub sentiment_score: f32
    }
}

pub struct FeatureStore {
    self_ref: FeatureStoreActor,
    system: SystemActor,
    extractor: FeatureExtractionManagerActor,
    db: PgConnection
}

#[derive_actor]
impl FeatureStore {
    pub fn train_model(&mut self, email: EmailBytes) {
        let self_ref = self.self_ref.clone();

        self.extractor.extract(email, std::sync::Arc::new(move |features| {
            match features {
                Ok(data) => {}
                Err(e) => {}
            };
        }));
    }

    //    pub fn store_sentiment_features(&self, Sen)
}

impl FeatureStore {
    pub fn new(extractor: FeatureExtractionManagerActor,
               self_ref: FeatureStoreActor,
               system: SystemActor) -> FeatureStore {
        FeatureStore {
            self_ref,
            system,
            extractor,
            db: FeatureStore::establish_connection(),
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