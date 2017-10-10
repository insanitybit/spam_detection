def load_truth():
    truth_map = {}

    with open('./TRAINING/SPAMTrain.label', 'r') as f:
        for line in f:
            if not line:
                continue
            truth = line[0]
            email = line[1:-1].strip()
            if truth == "1":
                truth = False
            else:
                truth = True
            truth_map[email] = truth
            # truth_map
    return truth_map

def predictions():
    pred_map = {}
    with open('./preds', 'r') as f:
        for line in f:
            if not line:
                continue

            name = line[12:27].strip()
            truth = line[29:]

            if truth == "true":
                truth = True
            else:
                truth = False
            pred_map[name] = truth

    return pred_map

if __name__ == '__main__':

    pred_map = predictions()
    truth_map = load_truth()

    for key in truth_map.keys():
        contains = pred_map.get(key)
        if contains is None:
            print(key)


    hits = 0
    misses = 0
    for file, truth in truth_map.items():
        try:
            pred = pred_map[file]
            if pred == truth:
                hits += 1
            else:
                misses += 1
        except:
            print("missed file: " + file)

    print(hits)
    print(misses)