from dataset import SelfplayDataModule
from train import parse_args
from model import NNUE
from dataset import CompositeFeaturesData
import pytorch_lightning as pl


args = parse_args()

trainer = pl.Trainer.from_argparse_args(
    args,
    auto_lr_find=True,
)

composite = CompositeFeaturesData(args.feature_set_tags)
model = NNUE(
    gamma=args.gamma,
    lr=args.lr,
    composite=composite,
)

datamodule = SelfplayDataModule(
    args.mongo_uri,
    args.tag,
    args.train_size,
    args.val_size,
    args.batch_size,
    args.prefetch_batches,
    args.mixer_size,
    composite,
    args.random_batch,
    pin_memory=isinstance(trainer.accelerator, pl.accelerators.CUDAAccelerator),
)

trainer.tune(model, datamodule=datamodule)
print(model.hparams['lr'])