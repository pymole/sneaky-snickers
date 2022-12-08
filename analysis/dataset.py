import numpy as np
from torch.utils.data import DataLoader, Dataset, random_split
import torch
import pipeline
import balalaika
import settings


class SelfplayDataset(Dataset):
    def __init__(self, size: int) -> None:
        # TODO: Use iterator of latest game logs to construct samples
        # TODO: Cache samples
        # TODO: Rotate, permutate
        # TODO: Pick by tags
        game_logs = pipeline.load_random_game_logs(10)
        self.samples, self.labels = prepare_examples(game_logs)
        
    def __len__(self):
        return len(self.samples)

    def __getitem__(self, index):
        return self.samples[index], self.labels[index]


def get_examples_from_game_log(game_log):
    _, boards, (board_rewards, _) = balalaika.rewind(game_log)
    
    xs = []
    ys = []

    # TODO: Get examples directly from balalaika
    for board in boards:
        examples = balalaika.get_examples(board, board_rewards)
        
        for indices, rewards in examples:
            # TODO: Make indices coalesed at the moment of parsing
            # TODO: C-structs
            features = prepare_feature_inidices(indices)
            xs.append(features)
            ys.append(torch.tensor(rewards))

    return xs, ys


def prepare_feature_inidices(indices):
    values = torch.ones(len(indices))
    indices = torch.Tensor(indices).unsqueeze(0)
    features = torch.sparse_coo_tensor(indices, values, size=(settings.FEATURES_COUNT,))
    return features


def prepare_examples(game_logs):
    xs = []
    ys = []

    for game_log in game_logs:
        x, y = get_examples_from_game_log(game_log)
        xs.extend(x)
        ys.extend(y)

    return xs, ys


def make_dataloaders(epoch_size, validation_size):
    dataset = SelfplayDataset(epoch_size + validation_size)
    # TODO: actual size
    train, val = random_split(dataset, [0.99, 0.01])
    train_dataloader = DataLoader(train)
    test_dataloader = DataLoader(val)
    return train_dataloader, test_dataloader
