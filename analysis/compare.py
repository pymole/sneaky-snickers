from sklearn.base import BaseEstimator
from sklearn.metrics import mean_absolute_error
from sklearn.linear_model import LinearRegression
import numpy as np
import balalaika

import pipeline


class FloodFillEstimator(BaseEstimator):
    def fit(self, **kwargs):
        return self

    def predict(self, X):
        Y = []
        for board in X:
            flood_fill = balalaika.flood_fill(board)
            Y.append(flood_fill[0])
        
        return Y


def compare(model1, model2, game_log):
    _, boards, (rewards, _) = balalaika.rewind(game_log)

    pred1 = model1.predict(boards)
    pred2 = model2.predict(boards)

    model1_name = type(model1).__name__
    model2_name = type(model2).__name__

    for i, board in enumerate(boards):
        balalaika.draw_board(board)
        
        print(f"Turn: {board['turn']}/{game_log['turns']}")
        print(f"{model1_name}: {pred1[i]}")
        print(f"{model2_name}: {pred2[i]}")

        input()

    rewards = [rewards[0]] * len(boards)

    scores1 = mean_absolute_error(pred1, rewards)
    scores2 = mean_absolute_error(pred2, rewards)

    print("Mean absolute error")
    print(f"{model1_name}. avg: {scores1.mean()} std: {scores1.std()}")
    print(f"{model2_name}. avg: {scores2.mean()} std: {scores2.std()}")


(test, *train) = pipeline.load_random_game_logs(5)

x, y = pipeline.prepare_examples(train)
xgboost = pipeline.fit_xgboost(x, y)

flood_fill = FloodFillEstimator()

compare(xgboost, flood_fill, test)