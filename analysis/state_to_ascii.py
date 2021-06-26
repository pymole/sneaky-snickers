#!/usr/bin/env python3

import json
import sys

from argparse import ArgumentParser
from pathlib import Path

# Example:
#
# A: health=100
# B: health=20
# ░░░░░░░░░░░░░░░░░░░░░░
# ░░░░ . . . . . . . . .
# ░░░░ . . . . . . ¤ . .
# ░░░░ b b B . . . . . .
# ░░░░ . . . . . . . . .
# ░░░░ . . . ¤ . . . . .
# ░░░░ . . . A . . . . .
# ░░░░ . . . a a a . . .
# ░░░░ . . . . . a . . .
# ░░░░ . . . . a a . . .
# ░░░░ . . . . . . . . .
# ░░░░ . . . . . . . . .

EMPTY = ' .'
HAZARD = '░░'
FOOD = ' ¤'
SNAKE_BODIES = [
    ' a',
    ' b',
    ' c',
    ' d',
    ' e',
    ' f',
    ' g',
    ' h',
]
SNAKE_HEADS = [
    ' A',
    ' B',
    ' C',
    ' D',
    ' E',
    ' F',
    ' G',
    ' H',
]


def to_ascii(state):
    header = []
    board = [
        [ EMPTY for _ in range(state['board']['width']) ]
        for _ in range(state['board']['height'])
    ]

    for p in state['board']['hazards']:
        board[p['y']][p['x']] = HAZARD

    for snake, head, body in zip(state['board']['snakes'], SNAKE_HEADS, SNAKE_BODIES):
        header.append(f'{head}: health={snake["health"]}')
        board[snake['head']['y']][snake['head']['x']] = head
        for p in snake['body'][1:]:
            board[p['y']][p['x']] = body

    for p in state['board']['food']:
        board[p['y']][p['x']] = FOOD

    board.reverse()

    return '\n'.join(header) + '\n' + '\n'.join(''.join(row) for row in board) + '\n'


def main():
    argparser = ArgumentParser(description='Convertd')
    argparser.add_argument(
        'state_file',
        nargs='?',
        type=Path,
        help='Path to json state file. If not given, stdin is used.'
    )
    args = argparser.parse_args()

    if args.state_file:
        print(to_ascii(json.load(open(args.state_file))))
    else:
        print(to_ascii(json.load(sys.stdin)))


if __name__ == '__main__':
    main()
