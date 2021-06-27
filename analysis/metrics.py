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
import requests
import dacite
from match import SnickersMatch, State, Point, snickers_state_to_battlesnake_turn


class BaseGameAnalyzer:
    def __init__(self, match: SnickersMatch):
        self.match = match
        self.size = match.game.width * match.game.height

        names = {}
        colors = {}
        for snake_meta in match.game.snakes_meta:
            names[snake_meta.id] = snake_meta.name
            colors[snake_meta.id] = snake_meta.color

        self.names = names
        self.colors = colors

    def analyze(self):
        estimates = defaultdict(list)
        for state in self.match.states:
            extra_data = self.extra_data(state)
            state_estimates = self.get_estimates(state, extra_data)
            for snake_id, estimate in state_estimates.items():
                estimates[snake_id].append(estimate)

        return self.get_figures(estimates)

    def get_estimates(self, state: State, extra_data) -> dict[str, float]:
        raise NotImplementedError

    def get_figures(self, estimates):
        raise NotImplementedError

    def extra_data(self, state: State):
        obstacles = set()
        for snake in state.snakes:
            if snake.health > 0:
                obstacles.update(snake.body)

        return {
            'obstacles': obstacles
        }

    def movement_positions(self, position: Point):
        if position.x < self.match.game.width - 1:
            yield Point(position.x + 1, position.y)
        if position.x > 0:
            yield Point(position.x - 1, position.y)
        if position.y < self.match.game.height - 1:
            yield Point(position.x, position.y + 1)
        if position.y > 0:
            yield Point(position.x, position.y - 1)


class LineGraphAnalyzer(BaseGameAnalyzer):
    def get_figures(self, estimates):
        fig = go.Figure()
        for snake_id, values in estimates.items():
            fig.add_trace(
                go.Scatter(
                    x=list(range(len(values))),
                    y=values,
                    line=dict(color=self.colors[snake_id]),
                    name=self.names[snake_id]
                )
            )

        fig.update_layout(
            xaxis_title="Turn",
            yaxis_title="State estimate",
            legend_title="Snakes",
        )

        return [fig]

    def get_estimates(self, state: State, extra_data: dict) -> dict[str, float]:
        raise NotImplementedError


class MovesAvailability(LineGraphAnalyzer):
    def get_estimates(self, state: State, extra_data: dict) -> dict[str, float]:
        estimates = {}
        for snake in state.snakes:
            if snake.health > 0:
                estimates[snake.id] = self.estimate_snake(
                    snake.body[0], snake.health, state.hazards, extra_data['obstacles'], state.food)

        return estimates

    def estimate_snake(self, head, health, hazards, obstacles, food):
        stack = [(head, health, 0)]
        visited = set()
        cost = 0

        while stack:
            point, health, level = stack.pop()

            if point in visited or health <= 0:
                continue

            visited.add(point)

            if point in food:
                health = 100

            reward = (self.size - level)
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

        return cost / (self.size ** 2)


class FloodFill(LineGraphAnalyzer):

    def get_figures(self, estimates):
        line_estimates = {
            name: [len(flood) / self.size for flood in values]
            for name, values in estimates.items()
        }

        line_figures = super().get_figures(line_estimates)

        rgb_colors = {snake_id: hex_to_rgb(color) for snake_id, color in self.colors.items()}

        flood_figure = go.Figure()
        turns = len(self.match.states)
        for turn in range(turns):
            matrix = np.full((self.match.game.height, self.match.game.width, 3), fill_value=(218, 232, 222))

            for snake_meta in self.match.game.snakes_meta:
                values = estimates[snake_meta.id]
                if turn < len(values):
                    for p in values[turn]:
                        matrix[p.y, p.x] = rgb_colors[snake_meta.id]

            for snake in self.match.states[turn].snakes:
                if snake.health <= 0:
                    continue

                for p in snake.body:
                    matrix[p.y, p.x] = rgb_colors[snake.id]
                    matrix[p.y, p.x] //= 2

                # Head is darker
                h = snake.body[0]
                matrix[h.y, h.x] //= 2

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
            active=0,
            currentvalue={'prefix': 'Turn: '},
            pad={'t': 50},
            steps=steps
        )]

        flood_figure.update_layout(
            sliders=sliders
        )

        return line_figures + [flood_figure]

    def get_estimates(self, state: State, extra_data: dict) -> dict[str, set]:
        obstacles = extra_data['obstacles']

        floods = {
            snake.id: (deque([snake.body[0]]), set())
            for snake in state.snakes
            if snake.health > 0
        }

        frontiers = set()

        while True:
            contested_points = defaultdict(list)
            for snake_id, (flood_front, visited) in floods.items():
                while flood_front:
                    point = flood_front.popleft()

                    for movement_position in self.movement_positions(point):
                        if movement_position in visited:
                            # Already seized this point
                            continue
                        if movement_position in frontiers:
                            # Can't seize frontiers
                            continue
                        if movement_position in obstacles:
                            # Body part is impassable
                            continue
                        if movement_position in contested_points and snake_id in contested_points[movement_position]:
                            # Already contesting for this point
                            continue

                        contested_points[movement_position].append(snake_id)

            all_fronts_empty = True
            for point, contenders in contested_points.items():
                if len(contenders) == 1:
                    snake_id = contenders[0]
                    flood_front, visited = floods[snake_id]
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


class FlavoredFloodFill(FloodFill):
    def extra_data(self, state: State):
        # Через сколько ходов частичка тела будет освобождена
        body_part_empty_at = {}
        sizes = {}
        for snake in state.snakes:
            if snake.health <= 0:
                continue

            for empty_at, body_part in enumerate(reversed(snake.body), start=1):
                body_part_empty_at[body_part] = empty_at

            sizes[snake.id] = len(snake.body)

        return {
            'body_part_empty_at': body_part_empty_at,
            'sizes': sizes,
        }

    def get_estimates(self, state: State, extra_data: dict) -> dict[str, list[Point]]:
        turn = snickers_state_to_battlesnake_turn(self.match.game, state)
        response = requests.post('http://localhost:8000/flavored_flood_fill', json=turn)
        return {
            snake_id: [dacite.from_dict(Point, point) for point in visited]
            for snake_id, visited in response.json().items()
        }