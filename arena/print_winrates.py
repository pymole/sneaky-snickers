#!/usr/bin/env python3

import json


with open('winrates.json', 'r') as f:
    winrates = json.load(f)


for player, opponents in winrates.items():
    winrate_sum = 0
    for opponent, player_wins in opponents.items():
        try:
            opponent_wins = winrates[opponent][player]
        except KeyError:
            opponent_wins = 0

        wins = player_wins + opponent_wins

        winrate = round(player_wins/wins * 100, 2)
        winrate_sum += winrate

        print(f'{player} vs {opponent}: {winrate}% ({player_wins}/{opponent_wins})')

    avg_winrate = round(winrate_sum/len(opponents), 2)
    print(f'{player} average: {avg_winrate}%\n')
