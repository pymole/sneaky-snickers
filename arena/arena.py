#!/usr/bin/env python3

from argparse import ArgumentParser
from dataclasses import dataclass
from pathlib import Path
from typing import Optional, NamedTuple
import _jsonnet
import atexit
import json
import logging
import random
import re
import shlex
import subprocess
import textwrap
import trueskill
import urllib.request


ARENA_DIR = Path(__file__).parent
ROOT_DIR = ARENA_DIR.parent
CONFIG_PATH = ARENA_DIR / 'config.jsonnet'


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
        assert len(self._addresses) > 0

    def __repr__(self) -> str:
        return f'BotUnmanaged(addresses={self._addresses})'

    def prepare(self) -> None:
        # Check if addresses are up
        for address in self._addresses:
            response = json.load(urllib.request.urlopen(address))
            if not response['apiversion'] == '1':
                raise Exception(f'Invalid apiversion on {address}')

    def up(self, ports_iter, copies=1) -> list[Address]:
        return self._addresses

    def down(self) -> None:
        return


class Player(NamedTuple):
    name: str
    address: Address


class Rules:
    RESULT_PATTERN : re.Pattern = re.compile(
        r'\[DONE\]: Game completed after \d+ turns. (.*) is the winner.|'
        r'\[DONE\]: Game completed after \d+ turns. It was a draw.'
    )

    def __init__(self, rules_config):
        self._build_dir : Path = Path(rules_config['build_dir'])
        self._engine : Path = (self._build_dir / 'official_engine').resolve()

    def prepare(self) -> None:
        run('go', 'build', '-o', self._engine, './cli/battlesnake/main.go', cwd=ARENA_DIR / 'rules')

    def play(self, players: list[Player]) -> list[int]:
        assert len(players) <= 8

        args = [
            str(self._engine),
            'play',
            '--width', '11',
            '--height', '11',
            '--gametype', 'royale'
        ]

        game_names = [f'{i}_{player.name}' for i, player in enumerate(players)]
        for name, player in zip(game_names, players):
            args += ['--name', name, '--url', player.address]

        logging.info(f'$ {shlex.join(args)}')

        r = subprocess.run(args, capture_output=True, check=False, text=True)
        # Note: This only distinguishes between winner or looser.
        winner = self._parse_log(r.stderr)
        return [ (0 if name == winner else 1) for name in game_names ]

    @staticmethod
    def _parse_log(log : str):
        return Rules.RESULT_PATTERN.search(log).group(1)


# Bot factory
def create_bot_from_config(bot_config) -> BotI:
    if bot_config['type'] == 'from_commit':
        return BotFromCommit(bot_config)
    elif bot_config['type'] == 'unmanaged':
        return BotUnmanaged(bot_config)

    return None


@dataclass
class RunningBot:
    name: str
    bot: BotI
    addresses: list[Address]


class RatingJsonEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, trueskill.Rating):
            return { 'mu': obj.mu, 'sigma': obj.sigma }
        return json.JSONEncoder.default(self, obj)


def load_ranks(filename) -> dict[str, trueskill.Rating]:
    return {
        name: trueskill.Rating(mu=rating['mu'], sigma=rating['sigma'])
        for name, rating in json.load(open(filename)).items()
    }


def dump_ranks(ranks, filename) -> None:
    json.dump(ranks, open(filename, 'w'), indent=4, cls=RatingJsonEncoder)


class Arena:
    def __init__(self, config):
        self._rules : Rules = Rules(config['rules'])
        self._bots : dict[str, BotI] = {
            name: bot
            for name, bot_config in config['bots'].items()
            if (bot := create_bot_from_config(bot_config)) is not None
        }
        self._running_bots : list[RunningBot] = None
        self._ranks_file : Path = Path(config['arena']['ranks_file'])
        self._ports_iter = iter(range(config['ports']['from'], config['ports']['to'] + 1))
        self._number_of_players : int = config['arena']['number_of_players']
        self._ladder_games : int = config['arena']['ladder_games']

    def prepare(self):
        self._rules.prepare()
        for bot in self._bots.values():
            bot.prepare()

    def up(self):
        self._running_bots = []
        for name, bot in self._bots.items():
            addresses = bot.up(self._ports_iter)
            assert len(addresses) > 0
            self._running_bots.append(RunningBot(name=name, bot=bot, addresses=addresses))

    # TODO: want to calculate win-rates
    def run_ladder(self):
        assert self._running_bots is not None, 'Call up() before run_ladder()'

        if self._number_of_players > len(self._running_bots):
            raise Exception(f'Not enough players to host {self._number_of_players}-players matches')

        ranks = load_ranks(self._ranks_file)
        for bot in self._running_bots:
            ranks.setdefault(bot.name, trueskill.Rating())

        logging.info(f'Running ladder for {self._ladder_games} games')

        for i in range(1, self._ladder_games + 1):
            selected_bots = random.sample(self._running_bots, k=self._number_of_players)
            players = [Player(name=bot.name, address=bot.addresses[0]) for bot in selected_bots]
            logging.info(f'[{i} / {self._ladder_games}] Starting game. Players: {players}')
            result = self._rules.play(players)
            # TODO: recalculate ranks
            logging.info(f'Result: {result}')

        dump_ranks(ranks, self._ranks_file)

    def down(self):
        assert self._running_bots is not None

        for running_bot in self._running_bots:
            running_bot.bot.down()

        self._running_bots = None


def main():
    logging.basicConfig(
        format='%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S',
        level=logging.INFO,
    )

    config = json.loads(_jsonnet.evaluate_file(str(CONFIG_PATH)))
    logging.info(f'Loaded config\n{textwrap.indent(json.dumps(config, indent=2), "    ")}')

    arena = Arena(config)
    arena.prepare()
    arena.up()
    arena.run_ladder()
    arena.down()


if __name__ == '__main__':
    main()
