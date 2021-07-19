#!/usr/bin/env python3

from argparse import ArgumentParser
from pathlib import Path
import subprocess
import json
import logging
import textwrap
import shlex
import atexit
from typing import Optional, NamedTuple


ARENA_DIR = Path(__file__).parent
ROOT_DIR = ARENA_DIR.parent
CONFIG_PATH = ARENA_DIR / 'config.json'


Address = str


def run(*args, **kwargs):
    logging.info(f'$ {shlex.join(map(str, args))} # {kwargs}')
    return subprocess.run(args, check=True, **kwargs)


class BotI:
    def prepare(self) -> None:
        raise NotImplementedError

    # TODO: rename to spawn?
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
        self._mute : bool = bot_config['mute']
        self._bot_processes : set[subprocess.Popen] = set()

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

        addresses = []
        for i, port in zip(range(copies), ports_iter):
            process = subprocess.Popen(
                [self._launch],
                cwd=self._build_dir,
                env={ 'ROCKET_PORT': str(port) },
                stderr=subprocess.DEVNULL if self._mute else None,
                stdout=subprocess.DEVNULL if self._mute else None
            )
            atexit.register(process.kill)
            self._bot_processes.add(process)
            addresses.append(f'http://127.0.0.1:{port}')

        return addresses

    def down(self) -> None:
        logging.info(f'{self}.down()')
        self._down()

    def _down(self) -> None:
        for p in self._bot_processes:
            p.terminate()

        while self._bot_processes:
            p = self._bot_processes.pop()

            try:
                p.wait(timeout=BotFromCommit.TERMINATE_TIMEOUT)
            except subprocess.TimeoutExpired:
                logging.info(
                    f"{self}.down(): Process didn't in {BotFromCommit.TERMINATE_TIMEOUT} seconds. Killing forcefully."
                )
                p.kill()
                p.wait()

            atexit.unregister(p.kill)

    def __del__(self):
        self._down()


class BotUnmanaged(BotI):
    def __init__(self, bot_config):
        assert bot_config['type'] == 'unmanaged'
        self._addresses : list[str] = bot_config['addresses']

    def __repr__(self) -> str:
        return f'BotUnmanaged(addresses={self._addresses})'

    def prepare(self) -> None:
        return

    def up(self, ports_iter, copies=1) -> list[Address]:
        return self._addresses

    def down(self) -> None:
        return


class Player(NamedTuple):
    name: str
    address: Address


class Rules:
    def __init__(self, rules_config):
        self._build_dir : Path = Path(rules_config['build_dir'])
        self._engine : Path = (self._build_dir / 'official_engine').resolve()

    def prepare(self) -> None:
        run('go', 'build', '-o', self._engine, './cli/battlesnake/main.go', cwd=ARENA_DIR / 'rules')

    def play(self, players: list[Player]) -> None:
        assert len(players) <= 8

        args = [
            str(self._engine),
            'play',
            '--width', '11',
            '--height', '11',
            '--gametype', 'royale'
        ]

        for i, player in enumerate(players):
            args += ['--name', f'{i}_{player.name}', '--url', player.address]

        logging.info(f'$ {shlex.join(args)}')

        r = subprocess.run(args, capture_output=True, check=False, text=True)
        import time; time.sleep(2)
        print('--------------------- stdout:')
        print(r.stdout)
        print('--------------------- stderr')
        print(r.stderr)
        print('---------------------')


def create_bot_from_config(bot_config) -> BotI:
    if bot_config['type'] == 'from_commit':
        return BotFromCommit(bot_config)
    elif bot_config['type'] == 'unmanaged':
        return BotUnmanaged(bot_config)

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

    rules = Rules(config['rules'])
    rules.prepare()

    ports = iter(range(config['ports']['from'], config['ports']['to']))
    bots['v1'].prepare()
    addresses = bots['v1'].up(ports)
    player_v1 = Player('v1', addresses[0])
    rules.play([player_v1, player_v1, player_v1, player_v1])
    import time; time.sleep(5)
    bots['v1'].down()

    if args.command == 'build':
        pass
    else:
        parser.print_help()


if __name__ == '__main__':
    main()
