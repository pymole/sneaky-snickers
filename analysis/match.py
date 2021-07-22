from dataclasses import dataclass, asdict


ME = 'sneaky-snickers'


@dataclass(unsafe_hash=True)
class Point:
    x: int
    y: int


@dataclass
class Snake:
    id: str
    body: list[Point]
    health: int


@dataclass
class State:
    turn: int
    snakes: list[Snake]
    hazards: list[Point]
    food: list[Point]


@dataclass
class Ruleset:
    damage_per_turn: int
    food_spawn_chance: int
    name: str
    minimum_food: int
    shrink_every_n_turns: int


@dataclass
class SnakeMeta:
    id: str
    name: str
    color: str


@dataclass
class Game:
    id: str
    width: int
    height: int
    snakes_meta: list[SnakeMeta]
    ruleset: Ruleset


@dataclass
class SnickersMatch:
    game: Game
    states: list[State]


def battlesnake_frames_to_snickers_match(data: dict) -> SnickersMatch:
    game = data['Game']['Game']
    game_id = game['ID']
    width = game['Width']
    height = game['Height']
    game_ruleset = game['Ruleset']
    ruleset = Ruleset(
        damage_per_turn=int(game_ruleset['damagePerTurn']),
        food_spawn_chance=int(game_ruleset['foodSpawnChance']),
        name=game_ruleset['name'],
        minimum_food=int(game_ruleset['minimumFood']),
        shrink_every_n_turns=int(game_ruleset['shrinkEveryNTurns']),
    )

    game_frames = data['Frames']
    meta = [
        SnakeMeta(id=snake['ID'], name=snake['Name'], color=snake['Color'])
        for snake in game_frames[0]['Snakes']
    ]

    states = []
    for game_frame in game_frames:
        snakes = [
            Snake(
                id=snake['ID'],
                body=[
                    Point(x=body_part['X'], y=body_part['Y'])
                    for body_part in snake['Body']
                ],
                health=0 if snake['Death'] else snake['Health'],
            )
            for snake in game_frame['Snakes']
        ]

        food = [
            Point(x=f['X'], y=f['Y'])
            for f in game_frame['Food']
        ]

        hazards = [
            Point(x=hazard['X'], y=hazard['Y'])
            for hazard in game_frame['Hazards']
        ]

        state = State(
            turn=game_frame['Turn'],
            snakes=snakes,
            hazards=hazards,
            food=food,
        )

        states.append(state)

    snickers_match = SnickersMatch(
        game=Game(
            id=game_id,
            width=width,
            height=height,
            snakes_meta=meta,
            ruleset=ruleset,
        ),
        states=states,
    )

    return snickers_match


def snickers_state_to_battlesnake_turn(game: Game, state: State):
    snake_id_to_name = { meta.id: meta.name for meta in game.snakes_meta }
    assert len(set(snake_id_to_name.values())) == len(snake_id_to_name), 'Non-unique snake names!'

    snakes = [
        {
            'id': snake.id,
            'name': snake_id_to_name[snake.id],
            'health': snake.health,
            'body': [asdict(body_part) for body_part in snake.body],
            'latency': '1',
            'head': asdict(snake.body[0]),
            'length': len(snake.body),
        }
        for snake in state.snakes
        if snake.health > 0
    ]

    snake_by_name = { snake['name']: snake for snake in snakes }

    battlesnake_turn = {
        'game': {
            'id': game.id,
            'ruleset': {
                'name': game.ruleset.name,
                'version': 'unknown',
            },
            # Random timeout
            'timeout': 500,
        },
        'turn': state.turn,
        'board': {
            'width': game.width,
            'height': game.height,
            'hazards': [asdict(hazard) for hazard in state.hazards],
            'food': [asdict(f) for f in state.food],
            'snakes': snakes,
        },
        'you': snake_by_name.get(ME, snakes[0]), # Prefer us if we are present, choose random snake otherwise
    }

    return battlesnake_turn
