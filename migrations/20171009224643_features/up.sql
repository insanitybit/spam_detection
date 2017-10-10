CREATE TABLE sentiment_features (
  id SERIAL PRIMARY KEY,
  hash bytea NOT NULL,
  positive_sentiment decimal NOT NULL,
  negative_sentiment decimal NOT NULL,
  sentiment_score decimal NOT NULL
);

CREATE TABLE truth_values (
  id SERIAL PRIMARY KEY,
  hash bytea NOT NULL,
  truth boolean NOT NULL
);
