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
        
        winrate = player_wins/wins
        winrate_sum += winrate
        
        print(player, 'vs', opponent + ':', winrate)
    
    print(player, 'average:', winrate_sum/len(opponents), '\n')
