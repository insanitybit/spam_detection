
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use schema::sentiment_features;
use schema::truth_values;

#[derive(Queryable)]
pub struct StoredSentimentFeatures {
    pub id: i32,
    pub hash: Vec<u8>,
    pub positive_sentiment: BigDecimal,
    pub negative_sentiment: BigDecimal,
    pub sentiment_score: BigDecimal
}

#[derive(Insertable)]
#[table_name = "sentiment_features"]
pub struct NewSentimentFeatures {
    pub hash: Vec<u8>,
    pub positive_sentiment: BigDecimal,
    pub negative_sentiment: BigDecimal,
    pub sentiment_score: BigDecimal
}

#[derive(Queryable)]
pub struct StoredTruthValue {
    pub id: i32,
    pub hash: Vec<u8>,
    pub truth: bool,
}

#[derive(Insertable)]
#[table_name = "truth_values"]
pub struct NewTruthValue {
    pub hash: Vec<u8>,
    pub truth: bool,
}
