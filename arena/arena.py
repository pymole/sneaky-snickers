#!/usr/bin/env python3

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Lock
from typing import Any, NamedTuple
import _jsonnet
import atexit
import concurrent.futures
import json
import logging
import numpy as np
import os
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
    # logging.info(f'$ {shlex.join(map(str, args))}') # TODO: Reduce kwargs and start logging them again.
    return subprocess.run(args, check=True, **kwargs)


class BotI:
    @property
    def name(self) -> str:
        raise NotImplementedError()

    @property
    def addresses(self) -> list[Address]:
        raise NotImplementedError()

    def prepare(self) -> None:
        raise NotImplementedError()

    def up(self, ports_iter, copies=1) -> None:
        raise NotImplementedError()

    def down(self) -> None:
        raise NotImplementedError()


class BotFromCommit(BotI):
    TERMINATE_TIMEOUT = 1

    def __init__(self, bot_config):
        assert bot_config['type'] == 'from_commit'

        self._name : str = bot_config['name']
        self._addresses : list[Address] = []

        self._build_dir    : Path      = Path(bot_config['build']['dir'])
        self._build_commit : str       = bot_config['build']['commit']
        self._build_flags  : list[str] = bot_config['build']['flags']

        self._run_exe  : str            = bot_config['run']['exe']
        self._run_env  : dict[str, str] = bot_config['run']['env']
        self._run_mute : bool           = bot_config['run']['mute']

        self._bot_processes : set[subprocess.Popen] = set()

    def __repr__(self) -> str:
        return f'BotFromCommit(commit={self._build_commit})'

    @property
    def name(self) -> str:
        return self._name

    @property
    def addresses(self) -> list[Address]:
        return self._addresses

    def prepare(self) -> None:
        logging.info(f'{self}.prepare()')

        if not self._build_dir.exists():
            self._build_dir.mkdir(parents=True)
            run('git', 'worktree', 'add', self._build_dir, self._build_commit)
        else:
            run('git', 'checkout', self._build_commit, cwd=self._build_dir)

        run(
            'cargo', 'build', *self._build_flags,
            cwd=self._build_dir,
            env=os.environ | { 'RUSTFLAGS': '-Awarnings' }
        )

    def up(self, ports_iter, copies=1) -> None:
        logging.info(f'{self}.up(copies={copies})')

        for _, port in zip(range(copies), ports_iter):
            process = subprocess.Popen(
                [self._run_exe],
                cwd=self._build_dir,
                env={ 'ROCKET_PORT': str(port) } | self._run_env,
                stderr=subprocess.DEVNULL if self._run_mute else None,
                stdout=subprocess.DEVNULL if self._run_mute else None
            )
            atexit.register(process.kill)
            self._bot_processes.add(process)
            self._addresses.append(f'http://127.0.0.1:{port}')

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

        self._addresses.clear()

    def __del__(self):
        self._down()


class BotUnmanaged(BotI):
    def __init__(self, bot_config):
        assert bot_config['type'] == 'unmanaged'
        self._name      : str       = bot_config['name']
        self._addresses : list[str] = bot_config['addresses']
        assert len(self._addresses) > 0

    def __repr__(self) -> str:
        return f'BotUnmanaged(addresses={self._addresses})'

    @property
    def name(self) -> str:
        return self._name

    @property
    def addresses(self) -> list[Address]:
        return self._addresses

    def prepare(self) -> None:
        return

    def up(self, ports_iter, copies=1) -> None:
        for address in self._addresses:
            response = json.load(urllib.request.urlopen(address))
            if not response['apiversion'] == '1':
                raise Exception(f'Invalid apiversion on {address}')

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
        self._warning_count = WithLock(0)

    @property
    def warning_count(self):
        return self._warning_count

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

        # logging.info(f'$ {shlex.join(args)}')

        r = subprocess.run(args, capture_output=True, check=False, text=True)

        for line in r.stderr.splitlines():
            if '[WARN]' in line:
                logging.warning(f'{line} (players={players})')
                with self._warning_count.lock:
                    self._warning_count.value += 1

        # Note: This only distinguishes between winner or looser.
        winner = self._parse_winner(r.stderr)
        logging.info(f'Winner: {winner}')
        return [ (0 if name == winner else 1) for name in game_names ]

    @staticmethod
    def _parse_winner(log : str):
        match = Rules.RESULT_PATTERN.search(log)
        if match is None:
            logging.error("Can't parse log.")
            print(log)
            raise Exception("Can't parse log.")
        return match.group(1)


# Bot factory
def create_bot_from_config(bot_config) -> BotI:
    if bot_config['type'] == 'from_commit':
        return BotFromCommit(bot_config)
    elif bot_config['type'] == 'unmanaged':
        return BotUnmanaged(bot_config)

    return None


class RatingJsonEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, trueskill.Rating):
            return { 'mu': obj.mu, 'sigma': obj.sigma }
        return json.JSONEncoder.default(self, obj)


def load_ratings(filename) -> dict[str, trueskill.Rating]:
    if not Path(filename).exists():
        return {}

    return {
        name: trueskill.Rating(mu=rating['mu'], sigma=rating['sigma'])
        for name, rating in json.load(open(filename)).items()
    }


def dump_ratings(ratings, filename) -> None:
    json.dump(ratings, open(filename, 'w'), indent=4, cls=RatingJsonEncoder)


def sample(xs : list[Any], k : int, weights : list[float], beta : float) -> list[Any]:
    assert len(weights) == len(xs)
    powered_weights = np.array(weights) ** beta
    probabilities = powered_weights / powered_weights.sum()
    rng = np.random.default_rng()
    return rng.choice(xs, size=k, replace=False, p=probabilities)


class WithLock:
    def __init__(self, value):
        self.lock = Lock()
        self.value = value


class Arena:
    def __init__(self, config):
        self._rules : Rules = Rules(config['rules'])
        self._bots : list[BotI] = [
            bot
            for bot_config in config['bots']
            if (bot := create_bot_from_config(bot_config)) is not None
        ]
        self._ratings_file : Path = Path(config['arena']['ratings_file'])
        self._ports_iter = iter(range(config['ports']['from'], config['ports']['to'] + 1))
        self._number_of_players : int = config['arena']['number_of_players']
        self._ladder_games : int = config['arena']['ladder_games']
        self._parallel : int = config['arena']['parallel']
        self._beta : float = config['arena']['beta']

    def prepare(self):
        self._rules.prepare()
        for bot in self._bots:
            bot.prepare()

    def up(self):
        for bot in self._bots:
            bot.up(self._ports_iter)
            assert len(bot.addresses) > 0

    def _run_random_game(self, i, weights) -> tuple[list[Player], list[int]]:
        try:
            with weights.lock:
                selected_bots = sample(
                    self._bots,
                    k=self._number_of_players,
                    weights=weights.value,
                    beta=self._beta,
                )

            players = [Player(name=bot.name, address=bot.addresses[0]) for bot in selected_bots]
            logging.info(
                f'[{i} / {self._ladder_games}] Starting game. Playing '
                + ' vs '.join(p.name for p in players)
                + f' (' + ', '.join(p.address for p in players) + ')'
                )
            ranks = self._rules.play(players)
            return players, ranks
        except Exception as e:
            logging.exception(e)
            raise

    # TODO: want to calculate win-rates
    def run_ladder(self):
        if self._number_of_players > len(self._bots):
            raise Exception(f'Not enough players to host {self._number_of_players}-players matches')

        ratings = load_ratings(self._ratings_file)
        for bot in self._bots:
            ratings.setdefault(bot.name, trueskill.Rating())

        compute_weights = lambda: [ ratings[bot.name].sigma for bot in self._bots ]

        logging.info(f'Running ladder for {self._ladder_games} games in {self._parallel} threads')

        with ThreadPoolExecutor(max_workers=self._parallel) as executor:
            completed = 0
            game_results = []

            try:
                weights = WithLock(compute_weights())
                game_results[:] = [
                    executor.submit(self._run_random_game, i, weights)
                    for i in range(1, self._ladder_games + 1)
                ]

                for game_result in concurrent.futures.as_completed(game_results):
                    assert game_result.done()

                    if game_result.cancelled() or game_result.exception() is not None:
                        continue

                    players, ranks = game_result.result()
                    new_ratings = trueskill.rate([(ratings[player.name],) for player in players], ranks=ranks)
                    for (new_rating,), player in zip(new_ratings, players):
                        ratings[player.name] = new_rating

                    with weights.lock:
                        weights.value[:] = compute_weights()

                    completed += 1

                    dump_ratings(ratings, self._ratings_file)


                executor.shutdown()
            except:
                executor.shutdown(cancel_futures=True)
                raise
            finally:
                logging.info(
                    f'completed {completed}, '
                    f'cancelled {sum(1 for f in game_results if f.cancelled())}, '
                    f'failed {sum(1 for f in game_results if not f.cancelled() and f.exception() is not None)}, '
                    f'warnings {self._rules.warning_count.value}'
                )
                dump_ratings(ratings, self._ratings_file)

    def down(self):
        for bot in self._bots:
            bot.down()


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
