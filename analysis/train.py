import pytorch_lightning as pl
import argparse
from dataset import CompositeFeaturesData
from model import NNUE
from pytorch_lightning import loggers as pl_loggers
from dataset import SelfplayDataModule


def parse_args():
    parser = argparse.ArgumentParser(description="Trains the network.")
    parser = pl.Trainer.add_argparse_args(parser)
    parser.add_argument("--gamma", default=0.992, type=float, help="Multiplicative factor applied to the learning rate after every epoch.")
    parser.add_argument("--lr", default=1e-4, type=float, help="Initial learning rate.")
    parser.add_argument("--batch-size", default=-1, type=int, help="Number of positions per batch / per iteration. Default on GPU = 8192 on CPU = 128.")
    parser.add_argument("--seed", default=42, type=int, help="torch seed to use.")
    parser.add_argument("--resume-from-model", help="Initializes training using the weights from the given .pt model")
    parser.add_argument("--network-save-period", type=int, default=20, help="Number of epochs between network snapshots. None to disable.")
    parser.add_argument("--save-last-network", action='store_true', help="Whether to always save the last produced network.")
    parser.add_argument("--train-size", type=int, default=2000, help="Number of game logs per epoch step.")
    parser.add_argument("--val-size", type=int, default=20, help="Number of game logs per validation step.")
    parser.add_argument("--prefetch-batches", type=int, default=20, help="Number of batches to prefetch.")
    parser.add_argument("--mixer-size", type=int, default=150000, help="Number of examples to store in memory.")
    parser.add_argument("--tag", type=str, required=True, help="Fetch game logs with this tag.")
    parser.add_argument("--feature-set-tags", nargs="*", help="Tags of feature sets.")
    parser.add_argument("--random-batch", action='store_true', help="Mix examples from different game logs in one batch.")
    parser.add_argument("--directory", type=str, required=False, help="Directory path to game logs. If provided use directory loader.")
    parser.add_argument("--mongo-uri", type=str, default="mongodb://battlesnake:battlesnake@localhost:27017/battlesnake?authSource=admin", help="MongoDB URI.")
    args = parser.parse_args()
    return args


if __name__ == '__main__':
    args = parse_args()

    batch_size = args.batch_size
    if batch_size <= 0:
        batch_size = 32

    if args.resume_from_model is None:
        composite = CompositeFeaturesData(args.feature_set_tags)
        model = NNUE(
            gamma=args.gamma,
            lr=args.lr,
            composite=composite,
        )
    else:
        model = NNUE.load_from_checkpoint(args.resume_from_model)
        composite = model.hparams["composite"]
        model.eval()

    pl.seed_everything(args.seed)
    print('Using batch size {}'.format(batch_size))

    logdir = args.default_root_dir if args.default_root_dir else 'logs/'
    print('Using log dir {}'.format(logdir), flush=True)

    tb_logger = pl_loggers.TensorBoardLogger(logdir)
    checkpoint_callback = pl.callbacks.ModelCheckpoint(
        save_last=args.save_last_network,
        every_n_epochs=args.network_save_period,
        save_top_k=-1,
    )
    trainer = pl.Trainer.from_argparse_args(
        args,
        callbacks=[checkpoint_callback],
        logger=tb_logger,
    )

    print(trainer.accelerator)

    datamodule = SelfplayDataModule(
        args.tag,
        args.train_size,
        args.val_size,
        args.batch_size,
        args.prefetch_batches,
        args.mixer_size,
        composite,
        args.random_batch,
        isinstance(trainer.accelerator, pl.accelerators.CUDAAccelerator),
        args.mongo_uri,
        args.directory,
    )

    trainer.fit(model, datamodule=datamodule)
