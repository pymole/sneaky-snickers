#!/usr/bin/env python3

from argparse import ArgumentParser
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import List, Set, Iterator, TypedDict, Union
from urllib.request import urlopen
import datetime
import json
import logging
import re

from match import SnickersMatch, snickers_state_to_battlesnake_turn, battlesnake_frames_to_snickers_match

GameId = str


ENGINE_HOST = 'https://engine.battlesnake.com'
ARENA_URL = 'https://play.battlesnake.com/arena/summer-league-2021'
RECENT_GAMES_REGEX = re.compile('href="/g/([^"]+)/"')


def download_frame(game_id: GameId, i: int):
    return json.load(urlopen(f'{ENGINE_HOST}/games/{game_id}/frames?offset={i}&limit=1'))['Frames'][0]

def download_frames_iter(game_id: GameId) -> Iterator[dict]:
    offset = 0

    while True:
        r = json.load(urlopen(f'{ENGINE_HOST}/games/{game_id}/frames?offset={offset}'))
        yield from r['Frames']

        if len(r['Frames']) == 0:
            return

        offset += len(r['Frames'])

def download_game(game_id: GameId) -> dict:
    return json.load(urlopen(f'{ENGINE_HOST}/games/{game_id}'))

def download(game_id: GameId) -> dict:
    return {
        'Game': download_game(game_id),
        'Frames': list(download_frames_iter(game_id)),
        'ScrapeTimestamp': datetime.datetime.now().astimezone().isoformat(),
    }

def download_battlesnake_turn(game_id: GameId, frame: int) -> dict:
    snickers_match = battlesnake_frames_to_snickers_match(
        {
            'Game': download_game(game_id),
            'Frames': [ download_frame(game_id, frame) ]
        }
    )
    return snickers_state_to_battlesnake_turn(snickers_match.game, snickers_match.states[0])

def list_recent_game_ids() -> List[GameId]:
    return RECENT_GAMES_REGEX.findall(urlopen(ARENA_URL).read().decode('utf-8'))


def get_stored_game_ids(storage_path: Path) -> Set[GameId]:
    return set(p.stem for p in storage_path.glob('*.json'))


def save_game(storage_path: Path, game_id: GameId, game: Union[dict, SnickersMatch]):
    json.dump(game, open(storage_path / f'{game_id}.json', 'wt'))


def parse_url_or_game_id(url_or_game_id: str) -> GameId:
    if url_or_game_id.startswith('https://'):
        game_id = url_or_game_id.removesuffix('/').split('/')[-1]
    else:
        game_id = url_or_game_id

    if not re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', game_id):
        raise Exception('Bad game id format')


    return game_id


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
    argparser_serve = subparsers.add_parser('serve', help='Run in server mode. Will scrape games on GET requests')
    argparser_game = subparsers.add_parser('game', help='Download game by its id')
    argparser_game.add_argument('game_id')
    argparser_game.add_argument('frame', nargs='?')
    argparser_game.add_argument('-o', '--output', required=False, default=Path('.'), help='Output folder', type=Path)
    args = argparser.parse_args()

    if args.command == 'game':
        game_id = parse_url_or_game_id(args.game_id)

        if args.frame is None:
            save_game(args.output, game_id, download(game_id))
        else:
            turn = download_battlesnake_turn(game_id, args.frame)
            json.dump(turn, open(args.output / f'{game_id}_{args.frame}.json', 'wt'))
    elif args.command == 'serve':
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                try:
                    url_or_game_id, frame = self.path.split('$')
                    url_or_game_id = url_or_game_id.removeprefix('/')
                    turn = download_battlesnake_turn(parse_url_or_game_id(url_or_game_id), frame)
                    self.send_response(200)
                    self.send_header('Content-type', 'application/json')
                    self.end_headers()
                    self.wfile.write(json.dumps(turn).encode('utf-8'))
                except:
                    logging.exception("Request failed")
                    self.send_response(404)
                    self.end_headers()

            def do_OPTIONS(self):
                self.send_response(200)
                self.end_headers()

            def end_headers(self):
                self.send_header('Access-Control-Allow-Origin', '*')
                self.send_header('Access-Control-Allow-Methods', 'POST, GET, PATCH, OPTIONS')
                self.send_header('Access-Control-Allow-Headers', '*')
                self.send_header('Access-Control-Allow-Credentials', 'true')
                super().end_headers()

        try:
            server = HTTPServer(('127.0.0.1', 8500), Handler)
            server.serve_forever()
        finally:
            server.server_close()

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
