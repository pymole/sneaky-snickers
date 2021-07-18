#!/usr/bin/env python3

from argparse import ArgumentParser
from pathlib import Path
import subprocess
import json
import logging
import textwrap
import shlex
from typing import Optional


ARENA_DIR = Path(__file__).parent
ROOT_DIR = ARENA_DIR.parent
CONFIG_PATH = ARENA_DIR / 'config.json'


Address = str


def run(*args, **kwargs):
    logging.info(f'$ {shlex.join(args)} # {kwargs}')
    return subprocess.run(args, check=True, **kwargs)


class BotI:
    def prepare(self) -> None:
        raise NotImplementedError

    def up(self, ports_iter, copies=1) -> list[Address]:
        raise NotImplementedError

    def down(self) -> None:
        raise NotImplementedError


class BotFromCommit(BotI):
    TERMINATE_TIMEOUT = 1

    def __init__(self, bot_config):
        assert bot_config['type'] == 'from_commit'
        self._build_dir : Path = Path(bot_config['build']['dir'])
        self._build_commit : str = bot_config['build']['commit']
        self._build_flags : list[str] = bot_config['build']['flags']
        self._launch : str = bot_config['launch']
        self._bot_process : Optional[subprocess.Popen] = None

    def __repr__(self) -> str:
        return f'BotFromCommit(commit={self._build_commit})'

    def prepare(self) -> None:
        logging.info(f'{self}.prepare()')

        if not self._build_dir.exists():
            self._build_dir.mkdir(parents=True)
            run('git', 'worktree', 'add', self._build_dir, self._build_commit)
        else:
            run('git', 'checkout', self._build_commit, cwd=self._build_dir)

        run('cargo', 'build', *self._build_flags, cwd=self._build_dir)

    def up(self, ports_iter, copies=1) -> list[Address]:
        logging.info(f'{self}.up(copies={copies})')
        port = next(ports_iter)
        self._bot_process = subprocess.Popen(
            [self._launch],
            cwd=self._build_dir,
            env={ 'ROCKET_PORT': str(port) }
        )

    def down(self) -> None:
        logging.info(f'{self}.down()')
        if self._bot_process is None:
            return

        if self._bot_process.returncode is not None:
            return

        self._bot_process.terminate()

        try:
            self._bot_process.wait(timeout=BotFromCommit.TERMINATE_TIMEOUT)
        except subprocess.TimeoutExpired:
            logging.info(
                f"{self}.down(): Process didn't in {BotFromCommit.TERMINATE_TIMEOUT} seconds. Killing forcefully."
            )
            self._bot_process.kill()
            self._bot_process.wait()


def create_bot_from_config(bot_config) -> BotI:
    if bot_config['type'] == 'from_commit':
        return BotFromCommit(bot_config)
    elif bot_config['type'] == 'external':
        return None # TODO

    return None


def main():
    logging.basicConfig(
        format='%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S',
        level=logging.INFO,
    )

    parser = ArgumentParser()
    subparsers = parser.add_subparsers(dest='command')
    build_parser = subparsers.add_parser('build', help='...')
    args = parser.parse_args()

    config = json.load(open(CONFIG_PATH))

    logging.info(f'Loaded config\n{textwrap.indent(json.dumps(config, indent=2), "    ")}')

    bots = {
        name: bot
        for name, bot_config in config['bots'].items()
        if (bot := create_bot_from_config(bot_config)) is not None
    }

    logging.info(bots)

    ports = iter(range(config['ports']['from'], config['ports']['to']))
    bots['v1'].prepare()
    bots['v1'].up(ports)
    import time; time.sleep(5)
    bots['v1'].down()

    if args.command == 'build':
        pass
    else:
        parser.print_help()


if __name__ == '__main__':
    main()
