import random
from dataclasses import dataclass


@dataclass
class RustInt:
    name: str
    diapason: tuple[int, int]


u32 = RustInt("u32", (0, 4294967295))
u64 = RustInt("u64", (0, 18446744073709551615))
u128 = RustInt("u128", (0, 340282366920938463463374607431768211455))



def generate(width: int, height: int, players: int, rust_integer: RustInt):
    low, high = rust_integer.diapason

    random_number = lambda: random.randint(low, high)

    food = [
        [0] * height
        for _ in range(width)
    ]
    body_directions = [
        [
            [
                [0] * 5 for _ in range(players)
            ]
            for _ in range(height)
        ]
        for _ in range(width)
    ]
    for x in range(width):
        for y in range(height):
            food[x][y] = random_number()

            for p in range(players):
                for direction in range(5):
                    body_directions[x][y][p][direction] = random_number()

    print(f"type ValueInt = {rust_integer.name};")
    print(f"const MAX_SNAKE_COUNT: usize = {players};")
    print(f"const MAX_WIDTH: usize = {width};")
    print(f"const MAX_HEIGHT: usize = {height};")

    food_str = "const FOOD: [[{}; MAX_HEIGHT]; MAX_WIDTH] = {};".format(
        rust_integer.name,
        food,
    )
    body_direcions_str = "const BODY_DIRECTIONS: [[[[{}; 5]; MAX_SNAKE_COUNT]; MAX_HEIGHT]; MAX_WIDTH] = {};".format(
        rust_integer.name,
        body_directions,
    )

    print(food_str)
    print(body_direcions_str)


if __name__ == "__main__":
    generate(11, 11, 4, u64)
