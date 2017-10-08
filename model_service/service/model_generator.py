import argparse
import pickle
import pandas as pd

from sklearn.ensemble import RandomForestClassifier

parser = argparse.ArgumentParser(description='Generates and stores a model.')
parser.add_argument('--data', type=str, help='Path to the extracted training data features.', required=True)
parser.add_argument('--output', type=str, help='Path to store the model.', required=True)


def get_args() -> (str, str):
    args = parser.parse_args()
    training = args.data
    output = args.output
    return training, output


def load_data(data_path: str) -> (pd.DataFrame, pd.Series):
    all_data = pd.read_csv(data_path)

    features = all_data.copy(deep=True).drop(['label'], axis=1)
    labels = pd.Series(all_data['label'])
    return features, labels


if __name__ == '__main__':
    data_path, output = get_args()

    features, labels = load_data(data_path)

    forest = RandomForestClassifier(n_estimators=10)
    forest = forest.fit(features, labels)

    with open(output, 'wb') as f:
        pickle.dump(forest, f)