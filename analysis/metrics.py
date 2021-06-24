"""
1. Тупики - это плохо, если свой хвост далеко
2. Голод ухудшает состояние экспоненциально. Он слабо влияет по
началу и сильно возрастает, когда здоровья остается совсем мало.
Хотя бывало и такое, что игрок в хорошем по голоду состоянии, нырял в хазард,
собрать несколько мороженок, чтобы просто прокачаться. Выходя оттуда он оставался на том же
здоровье, что и был, но уже прокаченный. Это было в дуэли.
3. Еда поблизости должна понижать влияние голода. Если у меня осталось
здоровья на один ход и рядом со мной находится еда, голод не должен понизить состояние.
4. Чем больше доступных действий поблизости, тем больше оценка.
+ В ближние точки можно добраться с большей вероятностью, чем в дальние.
5. Давать оценку клетки только на основе её удаленности наивно.
В удаленную клетку в ситуациях, когда тебе не мешают, можно добраться легко, а когда у противника.
Если контроль в той области, тогда сложно. Думаю, нужно оценивать клетку по доступности,
учитывая других игроков и голод.
6. В конце партии, когда места мало, кушать некогда и некуда.
Очень важно быть больше противника, чтобы была возможность давить,
создавать сложные для него ситуации.
"""

from collections import defaultdict, deque
import plotly.graph_objects as go
from plotly.colors import hex_to_rgb
import numpy as np


class BaseGameAnalyzer:
    def __init__(self, game):
        self.width = game['Game']['Game']['Width']
        self.height = game['Game']['Game']['Height']
        self.size = self.width * self.height
        self.colors = {snake['Name']: snake['Color'] for snake in game['Frames'][0]['Snakes']}

        states = []
        for frame in game['Frames']:
            food = {point_to_tuple(hazard) for hazard in frame['Food']}
            hazards = {point_to_tuple(hazard) for hazard in frame['Hazards']}

            snakes = {}
            obstacles = set()
            for snake in frame['Snakes']:
                if snake['Death'] is None:
                    body = [
                        point_to_tuple(point)
                        for point in snake['Body']
                    ]

                    obstacles.update(body)
                    snakes[snake['Name']] = (body, snake["Health"])

            states.append({
                'food': food,
                'hazards': hazards,
                'snakes': snakes,
                'obstacles': obstacles,
            })

        self.states = states

    def analyze(self):
        estimates = defaultdict(list)
        for state in self.states:
            # print(state['Turn'])
            state_estimates = self.get_estimates(state['snakes'], state['hazards'], state['obstacles'], state['food'])
            for snake_name, estimate in state_estimates.items():
                estimates[snake_name].append(estimate)

        return self.get_figures(estimates)

    def get_estimates(self, snakes_by_name, hazards, obstacles, food) -> dict[str, float]:
        raise NotImplementedError

    def get_figures(self, estimates):
        raise NotImplementedError

    def movement_positions(self, position):
        x, y = position
        if x < self.width - 1:
            yield x + 1, y
        if x > 0:
            yield x - 1, y
        if y < self.height - 1:
            yield x, y + 1
        if y > 0:
            yield x, y - 1


class LineGraphAnalyzer(BaseGameAnalyzer):
    def get_figures(self, estimates):
        fig = go.Figure()
        for name, values in estimates.items():
            fig.add_trace(
                go.Scatter(
                    x=list(range(len(values))),
                    y=values,
                    line=dict(color=self.colors[name]),
                    name=name
                )
            )

        fig.update_layout(
            xaxis_title="Turn",
            yaxis_title="State estimate",
            legend_title="Snakes",
        )

        return [fig]

    def get_estimates(self, snakes_by_name, hazards, obstacles, food) -> dict[str, float]:
        raise NotImplementedError


class MovesAvailability(LineGraphAnalyzer):
    def get_estimates(self, snakes_by_name, hazards, obstacles, food):
        estimates = {}
        for name, (body, health) in snakes_by_name.items():
            estimates[name] = self.estimate_snake(body[0], health, hazards, obstacles, food)

        return estimates

    def estimate_snake(self, head, health, hazards, obstacles, food):
        stack = [(head, health, 0)]
        visited = set()
        cost = 0
        size = self.width * self.height

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
                for movement_position in self.movement_positions(point)
                if movement_position not in visited and movement_position not in obstacles
            )

        return cost / (size ** 2)


class FloodFill(LineGraphAnalyzer):

    def get_figures(self, estimates):
        line_estimates = {
            name: [len(flood) / self.size for flood in values]
            for name, values in estimates.items()
        }

        line_figure = super().get_figures(line_estimates)

        rgb_colors = {name: hex_to_rgb(color) for name, color in self.colors.items()}

        flood_figure = go.Figure()
        turns = len(self.states)
        for turn in range(turns):
            matrix = np.full((self.height, self.width, 3), fill_value=(218, 232, 222))
            for name, values in estimates.items():
                if turn < len(values):
                    for x, y in values[turn]:
                        matrix[y, x] = rgb_colors[name]

                    state = self.states[turn]
                    body, _ = state['snakes'][name]
                    for x, y in body:
                        matrix[y, x] = rgb_colors[name]
                        matrix[y, x] //= 2

                    x, y = body[0]
                    matrix[y, x] //= 2

            matrix = np.flip(matrix, 0)

            flood_figure.add_trace(
                go.Image(
                    z=matrix,
                    visible=False,
                )
            )

        # Make 10th trace visible
        flood_figure.data[turns - 1].visible = True

        # Create and add slider
        steps = []
        for i in range(len(flood_figure.data)):
            step = dict(
                method='update',
                args=[
                    {'visible': [False] * len(flood_figure.data)},
                ],
            )
            step['args'][0]['visible'][i] = True
            steps.append(step)

        sliders = [dict(
            active=10,
            currentvalue={'prefix': 'Turn: '},
            pad={'t': 50},
            steps=steps
        )]

        flood_figure.update_layout(
            sliders=sliders
        )

        return line_figure + [flood_figure]

    def get_estimates(self, snakes_by_name, hazards, obstacles, food) -> dict[str, float]:
        floods = {
            name: (deque([body[0]]), set())
            for name, (body, health) in snakes_by_name.items()
        }

        frontiers = set()

        while True:
            contested_points = defaultdict(list)
            for name, (flood_front, visited) in floods.items():
                while flood_front:
                    point = flood_front.popleft()

                    for movement_position in self.movement_positions(point):
                        if (movement_position in visited
                                or movement_position in frontiers
                                or movement_position in contested_points and name in contested_points[movement_position]
                                or movement_position in obstacles):
                            continue

                        contested_points[movement_position].append(name)

            all_fronts_empty = True
            for point, contenders in contested_points.items():
                if len(contenders) == 1:
                    name = contenders[0]
                    flood_front, visited = floods[name]
                    flood_front.append(point)
                    visited.add(point)
                    all_fronts_empty = False
                else:
                    frontiers.add(point)

            if all_fronts_empty:
                break

        return {
            name: visited
            for name, (_, visited) in floods.items()
        }


def manhattan_distance(p0, p1):
    return abs(p0[0] - p1[0]) + abs(p0[1] - p1[1])


def point_to_tuple(point: dict) -> tuple:
    return point['X'], point['Y']
