from torch.utils.data import DataLoader, Dataset, random_split
import torch
import pipeline


class SelfplayDataset(Dataset):
    def __init__(self, size: int) -> None:
        # TODO: Use iterator of latest game logs to construct samples
        # TODO: Cache samples
        # TODO: Rotate, permutate
        # TODO: Pick by tags
        game_logs = pipeline.load_random_game_logs(200)
        self.samples, self.labels = pipeline.prepare_examples(game_logs)

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, index):
        return torch.from_numpy(self.samples[index]), torch.from_numpy(self.labels[index])


def make_dataloaders(epoch_size, validation_size):
    dataset = SelfplayDataset(epoch_size + validation_size)
    # TODO: actual size
    train, val = random_split(dataset, [0.99, 0.01])
    train_dataloader = DataLoader(train)
    test_dataloader = DataLoader(val)
    return train_dataloader, test_dataloader
