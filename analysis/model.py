import torch
import torch.nn as nn
import torch.nn.functional as F
import pytorch_lightning as pl
from pytorch_lightning.core.lightning import LightningModule
from dataset import BalalaikaBatch, CompositeFeaturesData
import settings


L2 = 128
L3 = 16
L4 = 32


class NNUE(pl.LightningModule):
    def __init__(self, gamma: float, lr: float, composite: CompositeFeaturesData):
        super().__init__()
        self.lr = lr
        self.gamma = gamma
        self.composite = composite
        self.layers = nn.Sequential(
            nn.Linear(composite.num_features, L2),
            nn.ReLU(),
            nn.Linear(L2, L3),
            nn.ReLU(),
            nn.Linear(L3, L4),
            nn.ReLU(),
            nn.Linear(L4, settings.SNAKES_COUNT),
            nn.Softmax(dim=0),
        )
        self.ce = nn.CrossEntropyLoss()
        self.save_hyperparameters()

    def forward(self, x):
        return self.layers(x)
  
    def training_step(self, batch: BalalaikaBatch, batch_idx):
        loss = self._step(batch, 'train_loss')
        return loss

    def validation_step(self, batch: BalalaikaBatch, batch_idx):
        loss = self._step(batch, 'val_loss')
        return loss
    
    def _step(self, batch: BalalaikaBatch, log_name):
        y_hat = self.layers(batch.x)
        loss = self.ce(y_hat, batch.y)
        self.log(log_name, loss, on_step=True, on_epoch=True, prog_bar=True, logger=True, batch_size=len(batch.x))
        return loss
    
    def configure_optimizers(self):
        optimizer = torch.optim.Adam(self.parameters(), lr=self.lr)
        scheduler = torch.optim.lr_scheduler.StepLR(optimizer, step_size=1, gamma=self.gamma)
        return [optimizer], [scheduler]
