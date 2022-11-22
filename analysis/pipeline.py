import xgboost
from sklearn.model_selection import cross_val_score
from sklearn.model_selection import RepeatedKFold
import numpy as np
import balalaika

from database import client
import settings


def load_random_game_logs(n: int):
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_logs = list(collection.aggregate([{
        "$sample": {
            "size": n
        }
    }]))
    return game_logs


def load_all_game_logs():
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_logs = collection.find()
    return game_logs


def swap_places(bool_grids, float_parameters, rewards):
    """
    Swap players' positions. Order of heads, bodies and rewards is the same:
    first player, second player.
    """
    
    bool_grids = bool_grids.copy()
    float_parameters = float_parameters.copy()
    rewards = rewards.copy()

    # Head
    bool_grids[3], bool_grids[4] = bool_grids[4], bool_grids[3]
    
    # Body
    bool_grids[5], bool_grids[6] = bool_grids[6], bool_grids[5]
    
    # Health
    float_parameters[0], float_parameters[1] = float_parameters[1], float_parameters[0]

    # Rewards
    # Save only one value, because with 2 players we have zero-sum game.
    # So me = 1 - opponent
    rewards[0], rewards[1] = rewards[1], rewards[0]

    return bool_grids, float_parameters, rewards


def convert_position(position):
    bool_grids, float_parameters = position
    bool_grids = np.array(bool_grids, np.bool8)
    float_parameters = np.array(float_parameters, np.float32)
    return bool_grids, float_parameters


def make_features(bool_grids, float_parameters):
    x = np.concatenate((bool_grids, float_parameters), axis=None)
    return x


def make_target(rewards):
    y = rewards[0]
    return y


def make_example(bool_grids, float_parameters, rewards):
    x = make_features(bool_grids, float_parameters)
    y = make_target(rewards)
    return x, y


def prepare_examples_from_game_log(game_log):
    positions, rewards = get_positions(game_log)
    rewards = np.array(rewards, np.float32)
    
    xs = []
    ys = []

    for position in positions:
        bool_grids, float_parameters = convert_position(position)
        swapped_bool_grids, swapped_float_parameters, swapped_rewards = swap_places(
            bool_grids, float_parameters, rewards
        )

        x, y = make_example(bool_grids, float_parameters, rewards)
        xs.append(x)
        ys.append(y)

        x, y = make_example(swapped_bool_grids, swapped_float_parameters, swapped_rewards)
        xs.append(x)
        ys.append(y)


    xs = np.array(xs)
    ys = np.array(ys)

    return xs, ys


def prepare_examples(game_logs):
    xs = []
    ys = []

    for game_log in game_logs:
        x, y = prepare_examples_from_game_log(game_log)
        xs.append(x)
        ys.append(y)

    xs = np.concatenate(xs)
    ys = np.concatenate(ys)

    return xs, ys


class BoardXGBRegressor(xgboost.XGBRegressor):
    def predict(self, X, **kwargs) -> np.ndarray:
        new_X = []
        for board in X:
            position = balalaika.get_position(board)
            bool_grids, float_parameters = convert_position(position)
            x = make_features(bool_grids, float_parameters)
            new_X.append(x)
        
        return super().predict(new_X, **kwargs)


def fit_xgboost(x, y) -> BoardXGBRegressor:
    model = BoardXGBRegressor()
    model.fit(x, y)
    return model
