import argparse
import pickle
import pandas as pd

from sklearn.ensemble import RandomForestClassifier
from sqlalchemy import create_engine

engine = create_engine('postgres://spam_detector:spam_detector@localhost/spam_detection')

parser = argparse.ArgumentParser(description='Generates and stores a model.')
parser.add_argument('--output', type=str, help='Path to store the model.', default="./model")


def load_data() -> (pd.DataFrame, pd.DataFrame):
    features = pd.read_sql_query('select * from "sentiment_features"', con=engine)
    labels = pd.read_sql_query('select * from "truth_values"', con=engine)

    features.drop(['id'], axis=1, inplace=True)
    labels.drop(['id'], axis=1, inplace=True)

    return features, labels


if __name__ == '__main__':
    features, labels = load_data()

    df = pd.merge(features, labels, on='hash', how='inner')

    features = df.copy(deep=True).drop(['truth', 'hash'], axis=1)
    labels = pd.Series(df['truth'])


    print(labels)
    forest = RandomForestClassifier(n_estimators=10)
    forest = forest.fit(features, labels)

    with open(parser.parse_args().output, 'wb') as f:
        pickle.dump(forest, f)