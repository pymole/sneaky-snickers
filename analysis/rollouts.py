import re
from collections import defaultdict


p = re.compile('.* sneaky_snickers::mcts - Started at (.*) turn and rolled out with (.*) turns and rewards (.*)')

with open("log.log") as f:
    data = f.read()


class TurnStat:
    def __init__(self):
        self.turn_sum = 0
        self.turns_count = 0
        self.max_turn = 0
        self.min_turn = float('inf')
        self.players_sum = 0


rollouts = p.findall(data)

turn_stats = defaultdict(TurnStat)
for start_turn, turn, rewards in rollouts:
    turn = int(turn)
    rewards = [float(r) for r in rewards[2:-2].split(', ')]
    
    stat = turn_stats[start_turn]
    
    stat.turn_sum += turn
    stat.turns_count += 1

    if turn > stat.max_turn:
        stat.max_turn = turn
    elif turn < stat.min_turn:
        stat.min_turn = turn

    stat.players_sum += len(rewards)


for turn, stat in turn_stats.items():
    avg_players = stat.players_sum / stat.turns_count
    print("Turn:", turn)
    print("Avg players:", avg_players)
    avg_turn = stat.turn_sum / stat.turns_count
    print("Avg turns count:", avg_turn)
    # print("Min turn count in rollouts:", stat.min_turn)
    # print("Max turn count:", stat.max_turn)

