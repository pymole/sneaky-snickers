"""
1. Тупики - это плохо, если свой хвост далеко
2. Голод ухудшает состояние экспоненциально. Он слабо влияет по
началу и сильно возрастает, когда здоровья остается совсем мало.
3. Еда поблизости должна понижать влияние голода. Если у меня осталось
здоровья на один ход и рядом со мной находится еда, голод не должен понизить состояние.
4. Чем больше доступных действий поблизости, тем больше оценка.
+ В ближние точки можно добраться с большей вероятностью, чем в дальние.
5. Давать оценку клетки только на основе её удаленности наивно.
В удаленную клетку в ситуациях, когда тебе не мешают, можно добраться легко, а когда у противника.
Если контроль в той области, тогда сложно. Думаю, нужно оценивать клетку по доступности,
учитывая других игроков и голод.
6. В конце партии, когда места мало, кушать некогда и некуда.
Очень важно быть больше противника, чтобы была возможность
создавать сложные для него ситуации.
"""

import argparse
import json
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator
from collections import defaultdict



def analize(frames_file):
    frames = json.load(frames_file)

    data = defaultdict(list)
    colors = {snake['Name']: snake['Color'] for snake in frames[0]['Snakes']}

    for frame in frames:
        print('turn:', frame['Turn'])
        snakes = frame['Snakes']
        hazards = {point_to_tuple(hazard) for hazard in frame['Hazards']}
        obstacles = set()
        food = {point_to_tuple(hazard) for hazard in frame['Food']}
        snakes_by_name = {}

        for snake in snakes:
            if snake['Death'] is None:
                body = [
                    point_to_tuple(point)
                    for point in snake['Body']
                ]

                obstacles.update(body)
                snakes_by_name[snake['Name']] = (body[0], snake["Health"])

        # for name, (head, health) in snakes_by_name.items():
        #     data[name].append(hunger_and_free_moves(food, health, head, obstacles, hazards, 11, 11))

        for name, (head, health) in snakes_by_name.items():
            data[name].append(moves_availability(food, health, head, obstacles, hazards, 11, 11))

    for name, values in data.items():
        plt.plot(values, label=name, color=colors[name])
    
    plt.legend(title='Snakes')
    plt.xlabel('Turn')
    plt.ylabel('Value')
    plt.grid(True)
    plt.gca().xaxis.set_major_locator(MaxNLocator(20, integer=True))
    plt.show()


def moves_availability(food, health, start, obstacles, hazards, width, height):
    stack = [(start, health, 0)]
    visited = set()
    cost = 0
    size = width * height

    while stack:
        point, health, level = stack.pop()

        if point in visited or health <= 0:
            continue

        visited.add(point)
        
        if point in food:
            health = 100

        reward = (size - level)
        if point in hazards:
            # reward /= 15
            health -= 15
        else:
            health -= 1
        
        cost += reward
        next_level = level + 1
        stack += (
            (movement_position, health, next_level)
            for movement_position in movement_positions(point, width, height)
            if movement_position not in visited and movement_position not in obstacles
        )
    
    # print(len(visited))

    return cost/(size ** 2)


def hunger_and_free_moves(food, health, start, obstacles, hazards, width, height):
    """Чем более голодная змейка, тем меньше влияния оказывает обилие ходов"""
    moves_k = 1

    move_availability = free_moves(start, obstacles, hazards, width, height)
    avg_food_distance = sum(manhattan_distance(start, f) for f in food)/len(food)/width/height
    hunger = 1.01 - (health/100)
    print(moves_k * move_availability, avg_food_distance, hunger)
    return avg_food_distance * move_availability/(2 ** hunger / 2)



def free_moves(start, obstacles, hazards, width, height):
    stack = [(start, 0)]
    visited = set()
    cost = 0
    size = width * height

    while stack:
        point, level = stack.pop()

        if point in visited:
            continue

        visited.add(point)
        
        reward = (size - level)
        if point in hazards:
            reward /= 15
        
        cost += reward
        next_level = level + 1
        stack += (
            (movement_position, next_level)
            for movement_position in movement_positions(point, width, height)
            if movement_position not in visited and movement_position not in obstacles
        )

    return cost/(size ** 2)


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


def manhattan_distance(p0, p1):
    return abs(p0[0] - p1[0]) + abs(p0[1] - p1[1])


def point_to_tuple(point: dict) -> tuple:
    return point['X'], point['Y']


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('frames', type=argparse.FileType('r'))
    analize(parser.parse_args().frames)
