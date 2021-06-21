#!/usr/bin/env python3

from argparse import ArgumentParser
from pathlib import Path
from typing import List, Set, Iterator
from urllib.request import urlopen
import datetime
import json
import logging
import re


GameId = str


ENGINE_HOST = 'https://engine.battlesnake.com'
ARENA_URL = 'https://play.battlesnake.com/arena/summer-league-2021'
RECENT_GAMES_REGEX = re.compile('href="/g/([^"]+)/"')


def download_frames_iter(game_id: GameId) -> Iterator[dict]:
    offset = 0

    while True:
        r = json.load(urlopen(f'{ENGINE_HOST}/games/{game_id}/frames?offset={offset}'))
        yield from r['Frames']

        if len(r['Frames']) == 0:
            return

        offset += len(r['Frames'])


def download(game_id: GameId) -> dict:
    game = json.load(urlopen(f'{ENGINE_HOST}/games/{game_id}'))
    frames = list(download_frames_iter(game_id))

    return {
        'Game': game,
        'Frames': frames,
        'ScrapeTimestamp': datetime.datetime.now().astimezone().isoformat(),
    }


def list_recent_game_ids() -> List[GameId]:
    return RECENT_GAMES_REGEX.findall(urlopen(ARENA_URL).read().decode('utf-8'))


def get_stored_game_ids(storage_path: Path) -> Set[GameId]:
    return set(p.stem for p in storage_path.glob('*.json'))


def main():
    logging.basicConfig(
        format='%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S',
        level=logging.INFO,
    )
    argparser = ArgumentParser()
    argparser.add_argument('--storage', required=True, help='Place to dump files', type=Path)
    args = argparser.parse_args()

    stored_game_ids = get_stored_game_ids(args.storage)
    logging.info(f'Found {len(stored_game_ids)} stored games.')

    recent_games = set(list_recent_game_ids())
    new_recent_games = recent_games - stored_game_ids
    n = len(new_recent_games)
    logging.info(f'Fetched {len(recent_games)} recent games. Found {n} new games.')

    for i, game_id in enumerate(new_recent_games, 1):
        if game_id in stored_game_ids:
            logging.info(f'[{i}/{n}] {game_id} is already downloaded.')
            continue
        json.dump(download(game_id), open(args.storage / f'{game_id}.json', 'wt'))
        stored_game_ids.add(game_id)
        logging.info(f'[{i}/{n}] Downloaded {game_id}.')


if __name__ == '__main__':
    main()
