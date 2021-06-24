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
    // `DoNothing` allows freezing some snakes in place.
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
                if board.squares[(x, y)].object == Object::Empty {
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

    // TODO: pub fn standard

    pub fn noop(_: &mut Board) {
    }
}

type SnakeStrategy<'a> = &'a mut dyn FnMut(/*snake_index:*/ usize, &Board) -> Action;

/// Dead snakes are kept in array to preserve indices of all other snakes
pub fn advance_one_step(board: &mut Board, engine_settings: &mut EngineSettings, snake_strategy: SnakeStrategy) {
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

        for (i, action) in alive_snakes.iter().copied().zip(actions) {
            if let Action::Move(movement) = action {
                let snake = &mut board.snakes[i];
                debug_assert!(snake.body.len() > 0);

                snake.body.push_front(snake.body[0] + movement.to_direction());
                let old_tail = snake.body.pop_back().unwrap();
                snake.health -= 1;

                debug_assert_eq!(board.squares[old_tail].object, Object::BodyPart);
                board.squares[old_tail].object = Object::Empty;
                // The head will be set in a separate loop.
            }
        }
    }

    // TODO: out of bounds head will
    let objects_under_head: ArrayVec<Object, MAX_SNAKE_COUNT> = alive_snakes.iter().copied()
        .map(|i| board.squares[board.snakes[i].body[0]].object)
        .collect();

    for i in alive_snakes.iter().copied() {
        board.squares[board.snakes[i].body[0]].object = Object::BodyPart;
    }

    // 2. Any Battlesnake that has found food will consume it:
    //     - Health reset set maximum.
    //     - Additional body part placed on top of current tail (this will extend their visible length by one on the
    //       next turn).
    //     - The food is removed from the board.
    {
        let mut eaten_food = ArrayVec::<Point, MAX_SNAKE_COUNT>::new();

        for (&i, &object_under_head) in alive_snakes.iter().zip(&objects_under_head) {
            let snake = &mut board.snakes[i];
            let head = snake.body[0];

            if object_under_head != Object::Food {
                continue;
            }

            snake.health = 100;

            let tail = *snake.body.back().unwrap();
            snake.body.push_back(tail);
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
        for &i in alive_snakes.iter() {
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

        for (&i, &object_under_head) in alive_snakes.iter().zip(&objects_under_head) {
            let snake = &board.snakes[i];
            let head = snake.body[0];

            let mut died = snake.health <= 0;
            died = died || !board.contains(head);
            died = died || object_under_head == Object::BodyPart;
            if !died {
                for j in alive_snakes.iter().copied() {
                    if i != j && head == board.snakes[j].body[0] {
                        died = true;
                        break;
                    }
                }
            }

            if died {
                died_snakes.push(i);
            }
        }

        for (&i, &object_under_head) in died_snakes.iter().zip(&objects_under_head) {
            board.snakes[i].health = 0;

            if object_under_head != Object::BodyPart {
                board.squares[board.snakes[i].body[0]].object = Object::Empty;
            }

            for &p in board.snakes[i].body.iter().skip(1) {
                board.squares[p].object = Object::Empty;
            }
        }

        // Restore heads, which would have been removed if it was a head-to-head collision.
        if died_snakes.len() > 0 {
            for &i in alive_snakes.iter() {
                let snake = &board.snakes[i];
                if snake.is_alive() {
                    board.squares[snake.body[0]].object = Object::BodyPart;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rocket::serde::json::serde_json;

    use crate::api;
    use crate::test_data as data;

    use super::*;

    fn create_board(api_str: &str) -> Board {
        Board::from_api(&serde_json::from_str::<api::objects::State>(api_str).unwrap())
    }

    fn test_transition(state_before: &str, state_after: &str, snake_strategy: SnakeStrategy) {
        let mut board_before = create_board(state_before);
        let board_after = create_board(state_after);
        let mut settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::noop,
        };

        advance_one_step(&mut board_before, &mut settings, snake_strategy);
        assert_eq!(board_before, board_after);
    }

    #[test]
    fn snake_moves_in_corect_direction() {
        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_UP,
            &mut |_, _| Action::Move(Movement::Up)
        );

        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_LEFT,
            &mut |_, _| Action::Move(Movement::Left)
        );

        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_RIGHT,
            &mut |_, _| Action::Move(Movement::Right)
        );

        {
            let mut board = create_board(data::SINGLE_SHORT_SNAKE_IN_THE_CENTER);
            let mut settings = EngineSettings {
                food_spawner: &mut food_spawner::noop,
                safe_zone_shrinker: &mut safe_zone_shrinker::noop,
            };

            advance_one_step(&mut board, &mut settings, &mut |_, _| Action::Move(Movement::Down));
            assert!(!board.snakes[0].is_alive(), "{:?}", &board.snakes[0]);

            for x in 0..board.size.x {
                for y in 0..board.size.y {
                    assert_eq!(board.squares[(x, y)].object, Object::Empty);
                }
            }
        }
    }

    #[test]
    fn health_depletes_by_one_on_each_move() {
        // TODO
    }

    #[test]
    fn snake_eats_food() {
        // TODO
    }

    #[test]
    fn snake_dies_out_of_bounds() {
        // TODO
    }

    #[test]
    fn snake_dies_from_self_collision() {
        // TODO
    }

    #[test]
    fn snake_dies_from_enemy_collision() {
        // TODO
    }

    #[test]
    fn two_snakes_in_head_to_head_collision() {
        // TODO
    }

    #[test]
    fn three_snakes_in_head_to_head_collision() {
        // TODO
    }

    #[test]
    fn four_snakes_in_head_to_head_collision() {
        // TODO
    }

    #[test]
    fn multiple_simultaneous_deaths() {
        // TODO
    }

    #[test]
    fn can_move_behind_tail() {
        // TODO
    }

    #[test]
    fn snake_dies_from_hunger() {
        // TODO
    }

    #[test]
    fn hazard_zone_depletes_hunger() {
        // TODO
    }

    #[test]
    fn food_eaten_in_hazard_zone() {
        // TODO
    }
}
