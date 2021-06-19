import argparse
import json
import matplotlib.pyplot as plt 
from collections import defaultdict


def analize(frames_file):
    frames = json.load(frames_file)

    data = defaultdict(list)

    for frame in frames:
        snakes = frame['Snakes']
        hazards = {point_to_tuple(hazard) for hazard in frame['Hazards']}
        obstacles = set()
        heads = {}
        for snake in snakes:
            if snake['Death'] is None:
                body = [
                    point_to_tuple(point)
                    for point in snake['Body']
                ]

                obstacles.update(body)
                heads[snake['Name']] = body[0]

        for name, head in heads.items():
            data[name].append(free_moves(head, obstacles, hazards, 11, 11))

        # for name, head in heads.items():
        #     data[name].append(moves_count_on_level(head, obstacles, hazards, 11, 11, 8))

    for name, values in data.items():
        plt.plot(values, label=name)
    
    plt.legend(title='Snakes')
    plt.xlabel('Turn')
    plt.ylabel('Value')
    plt.show()


def point_to_tuple(point: dict) -> tuple:
    return point['X'], point['Y']


def free_moves(start, obstacles, hazards, width, height):
    stack = [(start, 0)]
    visited = set()
    cost = 0

    while stack:
        point, level = stack.pop()

        if point in visited:
            continue

        visited.add(point)
        
        reward = width * height - level
        if point in hazards:
            reward /= 15
        
        cost += reward
        next_level = level + 1
        stack += (
            (movement_position, next_level)
            for movement_position in movement_positions(point, width, height)
            if movement_position not in visited and movement_position not in obstacles
        )

    return cost


def moves_count_on_level(start, obstacles, hazards, width, height, k):
    stack = [(start, 0)]
    visited = set()
    cost = 0

    while stack:
        point, level = stack.pop()

        if point in visited or level > k:
            continue

        visited.add(point)
        
        if level == k:
            reward = 1
            if point in hazards:
                reward /= 15
        
            cost += reward

        next_level = level + 1
        stack += (
            (movement_position, next_level)
            for movement_position in movement_positions(point, width, height)
            if movement_position not in visited and movement_position not in obstacles
        )

    return cost


def movement_positions(position, width, height):
    x, y = position
    if x < width - 1:
        yield (x + 1, y)
    if x > 0:
        yield (x - 1, y)
    if y < height - 1:
        yield (x, y + 1)
    if y > 0:
        yield (x, y - 1)



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('frames', type=argparse.FileType('r'))
    analize(parser.parse_args().frames)
