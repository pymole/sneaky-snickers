import sys
from model import NNUE


if __name__ == "__main__":
    (_, checkpoint_path, save_path) = sys.argv
    model = NNUE.load_from_checkpoint(checkpoint_path)
    model.to_torchscript(file_path=save_path)
