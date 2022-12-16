import random
from typing import List
from torch.utils.data import DataLoader, IterableDataset
import pytorch_lightning as pl
from pytorch_lightning.utilities.parsing import AttributeDict
import torch
import balalaika
import settings
from database import SelfplayRepository


FEATURE_SET_SIZES = balalaika.get_feature_set_sizes()


class CompositeFeaturesData(AttributeDict):
    def __init__(self, feature_set_tags: List[str], **kwargs) -> None:
        super().__init__(**kwargs)
        self['num_features'] = sum(FEATURE_SET_SIZES[tag] for tag in feature_set_tags)
        self['feature_sets'] = feature_set_tags


def prepare_features(indices, values, num_features):
    indices = torch.Tensor(indices).unsqueeze(0)
    values = torch.Tensor(values)
    # TODO: Make indices coalesed at the moment of parsing
    features = torch.sparse_coo_tensor(indices, values, size=(num_features,))
    return features


class BalalaikaBatch:
    def __init__(self, examples, num_features):
        xs = []
        ys = []
        for (indices, values), y in examples:
            xs.append(prepare_features(indices, values, num_features))
            ys.append(torch.Tensor(y))
        
        self.x = torch.stack(xs)
        self.y = torch.stack(ys)

    def pin_memory(self):
        # This is used by DataLoader to pin
        self.x.pin_memory()
        self.y.pin_memory()
        return self


class SelfplayDataset(IterableDataset):
    def __init__(
        self,
        mongo_uri: str,
        game_log_ids: List[int],
        batch_size: int,
        prefetch_batches: int,
        mixer_size: int,
        feature_sets: List[str],
        random_batch: bool
    ) -> None:
        self.mongo_uri = mongo_uri
        self.game_log_ids = game_log_ids
        self.batch_size = batch_size
        self.prefetch_batches = prefetch_batches
        self.mixer_size = mixer_size
        self.feature_sets = feature_sets
        self.random_batch = random_batch
    
    def __iter__(self):
        random.shuffle(self.game_log_ids)
        provider = balalaika.DataLoader(
            mongo_uri=self.mongo_uri,
            batch_size=self.batch_size,
            prefetch_batches=self.prefetch_batches,
            mixer_size=self.mixer_size,
            game_log_ids=self.game_log_ids,
            feature_set_tags=self.feature_sets,
            random_batch=self.random_batch,
        )
        return provider


def construct_collate_fn(composite: CompositeFeaturesData):
    def collate_fn(examples):
        return BalalaikaBatch(examples, composite.num_features)
    return collate_fn


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
        composite: CompositeFeaturesData,
        random_batch: bool,
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
        self.collate_fn = construct_collate_fn(composite)
        self.random_batch = random_batch
        self.composite = composite

    def setup(self, stage) -> None:
        game_log_ids = self.selfplay_repository.get_game_log_ids(
            self.tag,
            self.train_size + self.val_size,
        )
        assert len(game_log_ids) == self.train_size + self.val_size
        random.shuffle(game_log_ids)
        train_ids = game_log_ids[:self.train_size]
        test_ids = game_log_ids[self.train_size:]

        self.train_dataset = SelfplayDataset(
            settings.MONGO_URI, train_ids, self.batch_size, self.prefetch_batches,
            self.mixer_size, self.composite.feature_sets, self.random_batch,
        )
        self.val_dataset = SelfplayDataset(
            settings.MONGO_URI, test_ids, self.batch_size, self.prefetch_batches,
            self.mixer_size, self.composite.feature_sets, self.random_batch,
        )

    def train_dataloader(self):
        # TODO: Python workers
        train_dataloader = DataLoader(self.train_dataset, batch_size=None, batch_sampler=None, collate_fn=self.collate_fn, pin_memory=self.pin_memory)
        return train_dataloader

    def val_dataloader(self):
        val_dataloader = DataLoader(self.val_dataset, batch_size=None, batch_sampler=None, collate_fn=self.collate_fn, pin_memory=self.pin_memory)
        return val_dataloader
