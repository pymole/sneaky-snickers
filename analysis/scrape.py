#!/usr/bin/env python3

from argparse import ArgumentParser
from pathlib import Path
from typing import List, Set, Iterator, TypedDict, Union
from urllib.request import urlopen
import datetime
import json
import logging
import re

from match import SnickersMatch

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


def save_game(storage_path: Path, game_id: GameId, game: Union[dict, SnickersMatch]):
    json.dump(game, open(storage_path / f'{game_id}.json', 'wt'))


def main():
    logging.basicConfig(
        format='%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S',
        level=logging.INFO,
    )
    argparser = ArgumentParser()
    subparsers = argparser.add_subparsers(dest='command', required=True)
    argparser_recent = subparsers.add_parser('recent', help='Download all games referenced on "Recent Games" page')
    argparser_recent.add_argument('--storage', required=True, help='Place to dump files', type=Path)
    argparser_game = subparsers.add_parser('game', help='Download game by its id')
    argparser_game.add_argument('game_id')
    argparser_game.add_argument('-o', '--output', required=False, default=Path('.'), help='Output folder', type=Path)
    args = argparser.parse_args()

    if args.command == 'game':
        if args.game_id.startswith('https://'):
            game_id = args.game_id.removesuffix('/').split('/')[-1]
        else:
            game_id = args.game_id

        if not re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', game_id):
            raise Exception('Bad game id format')

        save_game(args.output, game_id, download(game_id))
    else:
        assert args.command == 'recent'

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

            save_game(args.storage, game_id, download(game_id))
            stored_game_ids.add(game_id)
            logging.info(f'[{i}/{n}] Downloaded {game_id}.')


if __name__ == '__main__':
    main()
