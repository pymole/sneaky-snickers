import xgboost
from sklearn.model_selection import cross_val_score
from sklearn.model_selection import RepeatedKFold
import numpy as np
import balalaika

from database import client
import settings


def load_game_log():
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_log = collection.find_one()
    return game_log


def load_all_game_logs():
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_logs = collection.find()
    return game_logs


def get_positions(game_log):
    _actions, positions, (rewards, _is_draw) = balalaika.get_positions(game_log)
    # [n, 11, 11, 11]
    return positions, rewards


def get_position_swaps(bool_grids, float_parameters, rewards):
    """
    Swap players' positions. Order of heads, bodies and rewards is the same:
    first player, second player.
    """
    
    bool_grids_swap = bool_grids.copy()
    # Head
    bool_grids_swap[3], bool_grids_swap[4] = bool_grids_swap[4], bool_grids_swap[3]
    # Body
    bool_grids_swap[5], bool_grids_swap[6] = bool_grids_swap[6], bool_grids_swap[5]

    float_parameters_swap = float_parameters.copy()
    # Health
    float_parameters_swap[0], float_parameters_swap[1] = float_parameters_swap[1], float_parameters_swap[0]

    xs = np.array([
        np.concatenate((bool_grids, float_parameters), axis=None),
        np.concatenate((bool_grids_swap, float_parameters_swap), axis=None),        
    ])

    # Save only one value, because with 2 players we have zero-sum game.
    # So me = 1 - opponent
    ys = np.array([
        rewards[0],
        rewards[1],
    ])

    return xs, ys


# game_logs = [load_game_log()]
game_logs = load_all_game_logs()[:20]
xs = []
ys = []

for game_log in game_logs:
    # print(game_log['_id'])
    positions, rewards = get_positions(game_log)
    rewards = np.array(rewards, np.float32)
    
    # print(len(positions), len(positions[0]))

    for position in positions:
        bool_grids, float_parameters = position

        bool_grids = np.array(bool_grids, np.bool8)
        float_parameters = np.array(float_parameters, np.float32)
        # print("INIT", bool_grids.shape, float_parameters.shape)
        # print("INIT", bool_grids, float_parameters)
        
        x, y = get_position_swaps(bool_grids, float_parameters, rewards)
        
        xs.append(x)
        ys.append(y)

xs = np.concatenate(xs)
ys = np.concatenate(ys)

print(xs.shape)
print(ys.shape)

# print(xs, xs.shape)
# print(ys, ys.shape)

# model = xgboost.XGBRegressor()
# cv = RepeatedKFold(n_splits=10, n_repeats=3, random_state=1)
# scores = cross_val_score(model, xs, ys, scoring='neg_mean_absolute_error', cv=cv, n_jobs=-1)
# scores = np.absolute(scores)
# print('Mean MAE: %.3f (%.3f)' % (scores.mean(), scores.std()) )
