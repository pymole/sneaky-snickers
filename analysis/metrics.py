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

from collections import defaultdict


class BaseGameAnalyzer:
    def __init__(self, game):
        self.width = game['Game']['Game']['Width']
        self.height = game['Game']['Game']['Height']
        self.states = game['Frames']

    def analyze_state(self, state):
        snakes = state['Snakes']
        hazards = {point_to_tuple(hazard) for hazard in state['Hazards']}
        obstacles = set()
        food = {point_to_tuple(hazard) for hazard in state['Food']}
        snakes_by_name = {}

        for snake in snakes:
            if snake['Death'] is None:
                body = [
                    point_to_tuple(point)
                    for point in snake['Body']
                ]

                obstacles.update(body)
                snakes_by_name[snake['Name']] = (body, snake["Health"])

        return self.get_estimates(snakes_by_name, hazards, obstacles, food)

    def analyze(self):
        estimates = defaultdict(list)
        for state in self.states:
            state_estimates = self.analyze_state(state)
            for snake_name, estimate in state_estimates.items():
                estimates[snake_name].append(estimate)

        return estimates

    def get_estimates(self, snakes_by_name, hazards, obstacles, food) -> dict[str, float]:
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


class MovesAvailability(BaseGameAnalyzer):
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

        return cost/(size ** 2)


class ProbabilisticMovesAvailability(BaseGameAnalyzer):
    """
    Глубина клетки - это длина кратчайшего пути до этой клетки от головы змейки.

    Маневренность целевой клеткой = вероятность того, что получится прийти в целевую клетку
    первым по самому короткому пути с условиями:
    - вражеские змейки могут преследовать любую из клеток на карте, с той же глубиной, что и целевая клетка
    - вражеские змейки достигают своих целей по кратчайшему пути
    - по мере прохождения к целевой клетке учитывать сбор еды
    - не умереть от голода
    - не столкться со змейкой, которая больше по размеру или с равным размером (размер змеи
    динамически увеличивается при поедании еды)
    - не столнуться с препятсвием (границы, тела)

    Маневренность = сумма маневренностей клеток деленая на количество клеток на поле.

    Маневренность показывает с какой вероятностью мы можем выбрать любую из клеток и достигнуть её.

    Более четко отображает владение полем. Максимизируя маневр, мы будем контроллировать пространство игры.
    Чем больше инструментов, для достижения своих целей, тем больше шансов на правильную победную реакцию
    на действия противника.
    """

    def get_estimates(self, snakes_by_name, hazards, obstacles, food):
        size = self.width * self.height

        for name, (body, health) in snakes_by_name:
            head = body[0]
            stack = [(head, health, 0)]
            visited = set()

            while stack:
                point, health, level = stack.pop()

                if point in visited or health <= 0:
                    continue

                visited.add(point)

                if point in food:
                    health = 100

                reward = (size - level)
                if point in hazards:
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

            for name, (body, health) in snakes_by_name.items():
                estimates[name] = self.estimate_snake(body[0], health, hazards, obstacles, food)

        return cost/(size ** 2)



def manhattan_distance(p0, p1):
    return abs(p0[0] - p1[0]) + abs(p0[1] - p1[1])


def point_to_tuple(point: dict) -> tuple:
    return point['X'], point['Y']
