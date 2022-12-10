from torch.utils.data import DataLoader, IterableDataset
import torch
import balalaika
import settings


def get_examples_from_game_log(game_log):
    examples = balalaika.get_examples_from_game_log(game_log)
    
    xs = []
    ys = []

    for indices, rewards in examples:
        features = prepare_feature_inidices(indices)
        xs.append(features)
        ys.append(torch.tensor(rewards))
    
    return xs, ys


def prepare_feature_inidices(indices):
    values = torch.ones(len(indices))
    indices = torch.Tensor(indices).unsqueeze(0)
    # TODO: Make indices coalesed at the moment of parsing
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


class SelfplayDataset(IterableDataset):
    def __init__(self, mongo_uri: str, batch_size: int = 128, prefetch_batches: int = 10, mixer_size: int = 80000) -> None:
        # TODO: Game log filtering
        self.provider = balalaika.DataLoader(
            mongo_uri=mongo_uri,
            batch_size=batch_size,
            prefetch_batches=prefetch_batches,
            mixer_size=mixer_size,
        )

    def __iter__(self):
        # TODO: Handle workers. Add indexing for workers.
        pass


# TODO Use LightningDataModule to refresh dataset at the end of epoch
def make_dataloaders(
    epoch_size,
    validation_size,
    batch_size,
):
    train_dataset = SelfplayDataset(settings.MONGO_URI, batch_size=batch_size)
    test_dataset = SelfplayDataset(settings.MONGO_URI, batch_size=batch_size)
    # TODO: actual size
    train_dataloader = DataLoader(train_dataset, batch_size=None)
    test_dataloader = DataLoader(test_dataset, batch_size=None)
    return train_dataloader, test_dataloader
