import random
from typing import List
from torch.utils.data import DataLoader, IterableDataset
import pytorch_lightning as pl
import torch
import balalaika
import settings
from database import SelfplayRepository


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


class BalalaikaBatch:
    def __init__(self, examples):
        xs = []
        ys = []
        for x, y in examples:
            xs.append(prepare_feature_inidices(x))
            ys.append(torch.tensor(y))
        
        self.x = torch.stack(xs)
        self.y = torch.stack(ys)

    def pin_memory(self):
        # This is used by DataLoader to pin
        self.x.pin_memory()
        self.y.pin_memory()
        return self


def collate_batch(examples):
    return BalalaikaBatch(examples)


class SelfplayDataset(IterableDataset):
    def __init__(
        self,
        mongo_uri: str,
        game_log_ids: List[int],
        batch_size: int,
        prefetch_batches: int,
        mixer_size: int,
    ) -> None:
        self.mongo_uri = mongo_uri
        self.game_log_ids = game_log_ids
        self.batch_size = batch_size
        self.prefetch_batches = prefetch_batches
        self.mixer_size = mixer_size
    
    def __iter__(self):
        random.shuffle(self.game_log_ids)
        provider = balalaika.DataLoader(
            mongo_uri=self.mongo_uri,
            batch_size=self.batch_size,
            prefetch_batches=self.prefetch_batches,
            mixer_size=self.mixer_size,
            game_log_ids=self.game_log_ids,
        )
        return provider


class SelfplayDataModule(pl.LightningDataModule):
    def __init__(
        self,
        mongo_uri: str,
        tag: str,
        train_size: int,
        val_size: int,
        batch_size: int,
        prefetch_batches: int,
        mixer_size: int,
        pin_memory: bool,
    ) -> None:
        super().__init__()
        self.selfplay_repository = SelfplayRepository(mongo_uri)
        self.batch_size = batch_size
        self.prefetch_batches = prefetch_batches
        self.mixer_size = mixer_size
        self.tag = tag
        self.train_size = train_size
        self.val_size = val_size
        self.pin_memory = pin_memory

    def setup(self, stage) -> None:
        game_log_ids = self.selfplay_repository.get_game_log_ids(
            self.tag,
            self.train_size + self.val_size,
        )
        assert len(game_log_ids) == self.train_size + self.val_size
        random.shuffle(game_log_ids)
        train_ids = game_log_ids[:self.train_size]
        test_ids = game_log_ids[self.train_size:]

        self.train_dataset = SelfplayDataset(settings.MONGO_URI, train_ids, self.batch_size, self.prefetch_batches, self.mixer_size)
        self.val_dataset = SelfplayDataset(settings.MONGO_URI, test_ids, self.batch_size, self.prefetch_batches, self.mixer_size)

    def train_dataloader(self):
        # TODO: Python workers
        train_dataloader = DataLoader(self.train_dataset, batch_size=None, batch_sampler=None, collate_fn=collate_batch, pin_memory=self.pin_memory)
        return train_dataloader

    def val_dataloader(self):
        val_dataloader = DataLoader(self.val_dataset, batch_size=None, batch_sampler=None, collate_fn=collate_batch, pin_memory=self.pin_memory)
        return val_dataloader
