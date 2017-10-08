import pandas as pd


from flask import Flask
from sklearn.ensemble import RandomForestClassifier
from io import StringIO


app = Flask(__name__)


def load_model(path) -> RandomForestClassifier:
    import pickle
    with open(path, 'rb') as f:
        forest = pickle.load(f)
    return forest


def get_args() -> str:
    model_path = "./model"
    return model_path

model_path = get_args()

forest = load_model(model_path)


@app.route('/predict/<string:csv_features>')
def predict(csv_features):
    print(csv_features)
    csv_features = StringIO(csv_features)
    features: pd.DataFrame = pd.read_csv(csv_features, names=['a','b','c'])

    p = forest.predict(features)
    print(p)
    return str(forest.predict(features)[0])