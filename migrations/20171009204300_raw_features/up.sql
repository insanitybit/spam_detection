CREATE TABLE sentiment_features (
  id SERIAL PRIMARY KEY,
  positive_sentiment decimal NOT NULL,
  negative_sentiment decimal NOT NULL,
  sentiment_score decimal NOT NULL
)