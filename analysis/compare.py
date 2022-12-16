import argparse
import torch
import balalaika

from model import NNUE
import database
import dataset
import settings


class FloodFillPredictor:
    def predict(self, board):
        return balalaika.flood_fill(board)


class NNUEPredictor:
    def __init__(self, model: NNUE):
        self.model = model

    def predict(self, board):
        indices, values = balalaika.get_features(board, self.model.composite.feature_sets)
        print(indices, values)
        features = dataset.prepare_features(indices, values, self.model.composite.num_features)
        
        x = self.model(features)
        return torch.round(x, decimals=2)


def compare(model1, model2, game_log):
    print(balalaika.inspect())
    _, boards, _ = balalaika.rewind(game_log)

    model1_name = type(model1).__name__
    model2_name = type(model2).__name__

    *boards, terminal_board = boards

    for board in boards:
        pred1 = model1.predict(board)
        pred2 = model2.predict(board)
        
        balalaika.draw_board(board)
        
        print(f"Turn: {board['turn']}/{game_log['turns']}")
        print(f"{model1_name}: {pred1}")
        print(f"{model2_name}: {pred2}")

        input()

    balalaika.draw_board(terminal_board)


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model1", type=str)
    parser.add_argument("--model2", type=str, required=False)
    return parser.parse_args()


if __name__ == "__main__":
    args = parse_args()
    model1 = NNUE.load_from_checkpoint(args.model1)
    model1.eval()
    model1 = NNUEPredictor(model1)

    if args.model2 is None:
        model2 = FloodFillPredictor()
    else:
        model2 = NNUE.load_from_checkpoint(args.model2)
        model2.eval()
        model2 = NNUEPredictor(model2)

    (game_log, ) = database.get_default_repo().load_random_game_logs(1)
    compare(model1, model2, game_log)
