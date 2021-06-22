use arrayvec::ArrayVec;

use crate::game::{
    Board,
    MAX_SNAKE_COUNT,
    Object,
    Point,
};

pub use crate::api::objects::Movement;

impl Movement {
    pub fn to_direction(self) -> Point {
        match self {
            Self::Right => Point {x:  1, y:  0},
            Self::Left  => Point {x: -1, y:  0},
            Self::Up    => Point {x:  0, y:  1},
            Self::Down  => Point {x:  0, y: -1},
        }
    }
}

pub enum Action {
    // `DoNothing` allows freezing some snakes in places and while running others
    DoNothing,
    Move(Movement),
}

pub struct EngineSettings<'a, 'b> {
    // Can append elements to `board.food`, but must not mutate anything else.
    pub food_spawner: &'a mut dyn FnMut(&mut Board),

    // Can shrink `board.safe_zone`, but must not mutate anything else.
    pub safe_zone_shrinker: &'b mut dyn FnMut(&mut Board),
}

pub mod food_spawner {
    use super::*;
    use rand;

    fn spawn_one(rng: &mut impl rand::Rng, board: &mut Board) {
        let empty_squares_count = board.squares.data.iter().filter(|s| s.object == Object::Empty).count();

        if empty_squares_count == 0 {
            return
        }

        let needle = rng.gen_range(0..empty_squares_count);

        let mut i = 0;
        for x in 0..board.squares.len1 {
            for y in 0..board.squares.len2 {
                if let Object::Empty = board.squares[(x, y)].object {
                    if i == needle {
                        board.squares[(x, y)].object = Object::Food;
                        board.foods.push(Point { x: x as i32, y: y as i32 });
                        return;
                    }

                    i += 1;
                }
            }
        }

        unreachable!();
    }

    pub fn create_standard(mut rng: impl rand::Rng) -> impl FnMut(&mut Board) {
        move |board: &mut Board| {
            if board.foods.len() < 1 || rng.gen_ratio(20, 100) {
                spawn_one(&mut rng, board);
            }
        }
    }

    pub fn noop(_: &mut Board) {
    }
}

pub mod safe_zone_shrinker {
    use super::*;

    // pub fn standard

    pub fn noop(_: &mut Board) {
    }
}

/// Dead snakes are kept in array to preserve indices of all other snakes
/// WIP
pub fn advance_one_step(
    board: &mut Board,
    engine_settings: &mut EngineSettings,
    snake_strategy: &mut dyn FnMut(/*snake_index:*/ usize, &Board) -> Action,
)
{
    board.turn += 1;

    let alive_snakes: ArrayVec<usize, MAX_SNAKE_COUNT> = (0..board.snakes.len())
        .filter(|&i| board.snakes[i].is_alive())
        .collect();

    debug_assert!(
        alive_snakes.iter().all(|&i| board.snakes[i].body.len() > 0)
    );

    // From https://docs.battlesnake.com/references/rules
    // 1. Each Battlesnake will have its chosen move applied:
    //     - A new body part is added to the board in the direction they moved.
    //     - Last body part (their tail) is removed from the board.
    //     - Health is reduced by 1.
    {
        let actions: ArrayVec<Action, MAX_SNAKE_COUNT> = alive_snakes.iter()
            .map(|&i| snake_strategy(i, &board))
            .collect();

        // TODO: bug with spawn_turn when snake is not moving. Is this field actually needed? It can be computed
        // separately, if needed
        for (i, action) in alive_snakes.iter().copied().zip(actions) {
            if let Action::Move(movement) = action {
                let snake = &mut board.snakes[i];
                debug_assert!(snake.body.len() > 0);

                snake.body.push_front(snake.body[0] + movement.to_direction());
                let old_tail = snake.body.pop_back().unwrap();
                snake.health -= 1;

                debug_assert_eq!(board.squares[old_tail].object, Object::BodyPart);

                board.squares[old_tail].object = Object::Empty;
                // TODO: wrong! this conflicts with next step of consuming food
                board.squares[snake.body[0]].object = Object::BodyPart;
            }
        }
    }

    // 2. Any Battlesnake that has found food will consume it:
    //     - Health reset set maximum.
    //     - Additional body part placed on top of current tail (this will extend their visible length by one on the
    //       next turn).
    //     - The food is removed from the board.
    {
        let mut eaten_food = ArrayVec::<Point, MAX_SNAKE_COUNT>::new();

        for i in alive_snakes.iter().copied() {
            let head = board.snakes[i].body[0];

            if board.squares[head].object != Object::Food {
                continue;
            }

            board.snakes[i].health = 100;

            let tail = *board.snakes[i].body.back().unwrap();
            board.snakes[i].body.push_back(tail);
            debug_assert_eq!(board.squares[tail].object, Object::BodyPart);
            eaten_food.push(head);
        }

        for food in eaten_food {
            board.foods.swap_remove(board.foods.iter().position(|&x| x == food).unwrap());
            board.squares[food].object = Object::Empty;
        }
    }

    // 3. Any new food spawning will be placed in empty squares on the board.
    {
        (engine_settings.food_spawner)(board);
    }

    // Battle Royale ruleset. Do in this order:
    // - Deal out-of-safe-zone damage
    // - Maybe shrink safe zone
    {
        for i in alive_snakes.iter().copied() {
            if !board.safe_zone.contains(board.snakes[i].body[0]) {
                board.snakes[i].health -= 15;
            }
        }

        (engine_settings.safe_zone_shrinker)(board);
    }

    // 4. Any Battlesnake that has been eliminated is removed from the game board:
    //     - Health less than or equal to 0
    //     - Moved out of bounds
    //     - Collided with themselves
    //     - Collided with another Battlesnake
    //     - Collided head-to-head and lost
    {
        let mut died_snakes = ArrayVec::<usize, MAX_SNAKE_COUNT>::new();

        for i in alive_snakes.iter().copied() {
            let snake = &board.snakes[i];

            let mut died = snake.health <= 0;
            died = died || !board.contains(snake.body[0]);
            died = died || matches!(board.squares[snake.body[0]].object, Object::BodyPart {..});
            if !died {
                for j in alive_snakes.iter().copied() {
                    if i != j && snake.body[0] == board.snakes[j].body[0] {
                        died = true;
                        break;
                    }
                }
            }

            if died {
                died_snakes.push(i);
            }
        }

        for i in died_snakes {
            board.snakes[i].health = 0;
            for p in board.snakes[i].body.iter().copied() {
                // TODO: Wait, this is wrong in case of body collision
                board.squares[p].object = Object::Empty;
            }
        }

        // In case we removed alive head, here we restore it.
        for i in alive_snakes.iter().copied() {
            if board.snakes[i].is_alive() {
                board.squares[board.snakes[i].body[0]].object = Object::BodyPart;
            }
        }
    }

}
