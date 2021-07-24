use arrayvec::ArrayVec;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use crate::game::{
    Board,
    MAX_SNAKE_COUNT,
    Object,
    Point,
};

pub use crate::api::objects::Movement;

pub const MOVEMENTS: [Movement; 4] = [
    Movement::Up,
    Movement::Right,
    Movement::Down,
    Movement::Left,
];

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

impl Distribution<Movement> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Movement {
        match rng.gen_range(0..4) {
            0 => Movement::Down,
            1 => Movement::Up,
            2 => Movement::Right,
            3 => Movement::Left,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    // `DoNothing` allows freezing some snakes in place.
    // Not implemented, until use case is clear. Right now it is not clear, if health should deplete and how
    // head-to-head collisions with frozen snake should be resolved.
    // DoNothing,
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
        let empty_objects_count = board.objects.data.iter().filter(|&&object| object == Object::Empty).count();

        if empty_objects_count == 0 {
            return;
        }

        let needle = rng.gen_range(0..empty_objects_count);

        let mut i = 0;
        for x in 0..board.objects.len1 {
            for y in 0..board.objects.len2 {
                if board.objects[(x, y)] == Object::Empty {
                    if i == needle {
                        board.objects[(x, y)] = Object::Food;
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

    pub fn shrink(board: &mut Board, side: Movement) {
        if board.safe_zone.empty() {
            return;
        }
        match side {
            Movement::Left => board.safe_zone.p0.x += 1,
            Movement::Up => board.safe_zone.p0.y -= 1,
            Movement::Right => board.safe_zone.p1.x -= 1,
            Movement::Down => board.safe_zone.p1.y += 1,
        }
    }

    // TODO: pub fn standard
    pub fn standard(board: &mut Board) {
        if board.turn != 0 && board.turn % 20 == 0 {
            let side: Movement = rand::random();
            shrink(board, side);
        }
    }

    pub fn noop(_: &mut Board) {
    }
}

type SnakeStrategy<'a> = &'a mut dyn FnMut(/*snake_index:*/ usize, &Board) -> Action;

pub fn advance_one_step(board: &mut Board, snake_strategy: SnakeStrategy) -> Vec<(usize, Action)> {
    let mut settings = EngineSettings {
        food_spawner: &mut food_spawner::noop,
        safe_zone_shrinker: &mut safe_zone_shrinker::noop,
    };

    advance_one_step_with_settings(board, &mut settings, snake_strategy)
}

/// Dead snakes are kept in array to preserve indices of all other snakes
pub fn advance_one_step_with_settings(
    board: &mut Board,
    engine_settings: &mut EngineSettings,
    snake_strategy: SnakeStrategy
) -> Vec<(usize, Action)> {
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

    let actions: Vec<(usize, Action)> = alive_snakes
        .iter()
        .map(|&i| (i, snake_strategy(i, &board)))
        .collect();

    {
        for &(i, action) in actions.iter() {
            let Action::Move(movement) = action;

            let snake = &mut board.snakes[i];
            debug_assert!(snake.body.len() > 0);

            snake.body.push_front(snake.body[0] + movement.to_direction());
            let old_tail = snake.body.pop_back().unwrap();
            snake.health -= 1;

            debug_assert_eq!(board.objects[old_tail], Object::BodyPart);

            // TODO: benchmark alternative: if *snake.body.back().unwrap() != old_tail {
            board.objects[old_tail] = Object::Empty;
            board.objects[*snake.body.back().unwrap()] = Object::BodyPart;

            // The head will be set in a separate loop.
        }
    }

    let mut objects_under_head = ArrayVec::<Object, MAX_SNAKE_COUNT>::new();

    for &i in alive_snakes.iter() {
        let head = board.snakes[i].body[0];
        if board.contains(head) {
            objects_under_head.push(board.objects[head]);
        } else {
            // Note: Object::Empty will also work in current implementation
            // Note: If board.objects would allow out of bounds access, only "then" branch will suffice.
            objects_under_head.push(Object::BodyPart);
        }
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
            debug_assert_eq!(board.objects[tail], Object::BodyPart);
            eaten_food.push(head);
        }

        for food in eaten_food {
            if let Some(food_position) = board.foods.iter().position(|&x| x == food) {
                board.foods.swap_remove(food_position);
                board.objects[food] = Object::Empty;
            }
        }
    }

    // 3. Any new food spawning will be placed in empty objects on the board.
    {
        (engine_settings.food_spawner)(board);
    }

    // 4. Any Battlesnake that has been eliminated is removed from the game board:
    //     - Health less than or equal to 0
    //     - Moved out of bounds
    {
        for &i in alive_snakes.iter() {
            if !board.contains(board.snakes[i].body[0]) {
                board.snakes[i].health = 0;
            }
        }
    }

    //     - Collided with themselves
    //     - Collided with another Battlesnake
    //     - Collided head-to-head and lost
    {

        let mut died_snakes = ArrayVec::<usize, MAX_SNAKE_COUNT>::new();

        for (&i, &object_under_head) in alive_snakes.iter().zip(&objects_under_head) {

            if !board.snakes[i].is_alive() {
                continue;
            }

            // Collided with themselves or another battlesnake.
            if object_under_head == Object::BodyPart {
                died_snakes.push(i);
                continue;
            }

            // Head-to-head.
            for j in alive_snakes.iter().copied() {
                let body_i = &board.snakes[i].body;
                let body_j = &board.snakes[j].body;

                if i != j && body_i[0] == body_j[0] && body_i.len() <= body_j.len() && board.snakes[j].is_alive() {
                    died_snakes.push(i);
                    break;
                }
            }
        }

        for &i in died_snakes.iter() {
            board.snakes[i].health = 0;
        }
    }

    // At last, aplly Battle Royale ruleset. Do in this order:
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

    // Remove dead snakes from board.objects.
    {
        for (&i, &object_under_head) in alive_snakes.iter().zip(&objects_under_head) {
            if !board.snakes[i].is_alive() {

                board.snakes[i].health = 0;

                if object_under_head != Object::BodyPart {
                    board.objects[board.snakes[i].body[0]] = Object::Empty;
                }

                for &p in board.snakes[i].body.iter().skip(1) {
                    board.objects[p] = Object::Empty;
                }
            }
        }

        // Restore alive heads, which were removed in previous loop in case of a head-to-head collision.
        for &i in alive_snakes.iter() {
            if board.snakes[i].is_alive() {
                board.objects[board.snakes[i].body[0]] = Object::BodyPart;
            }
        }
    }

    actions
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
        advance_one_step(&mut board_before, snake_strategy);
        assert_eq!(board_before, board_after);
    }

    fn is_empty(board: &Board) -> bool {
        for x in 0..board.size.x {
            for y in 0..board.size.y {
                if board.objects[(x, y)] != Object::Empty {
                    return false;
                }
            }
        }

        return true;
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
            advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Down));
            assert!(!board.snakes[0].is_alive(), "{:?}", &board.snakes[0]);
            assert!(is_empty(&board));
        }
    }

    #[test]
    fn health_depletes_by_one_on_each_move() {
        let mut board = create_board(data::SINGLE_SHORT_SNAKE_IN_THE_CENTER);
        assert_eq!(board.snakes[0].health, 100);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
        assert_eq!(board.snakes[0].health, 99);
    }

    #[test]
    fn snake_eats_food() {
        let mut board = create_board(data::FOOD_IN_FRONT);
        assert_eq!(board.foods.len(), 3);
        assert_eq!(board.snakes[0].body.len(), 4);
        let food_point = board.foods[1];
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
        assert_eq!(board.snakes[0].body[0], food_point);
        assert_eq!(board.foods.len(), 2);
        assert_eq!(board.snakes[0].body.len(), 5);
        assert_eq!(board.snakes[0].body[3], board.snakes[0].body[4]);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
        assert_eq!(board, create_board(data::FOOD_IN_FRONT_UP_UP));
    }

    #[test]
    fn snake_dies_out_of_bounds() {
        {
            let mut board = create_board(data::TOP_RIGHT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::TOP_RIGHT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Right));
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::BOTTOM_LEFT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Left));
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::BOTTOM_LEFT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Down));
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }
    }

    #[test]
    fn snake_dies_from_self_collision() {
        let mut board = create_board(data::STEP_ON_TAIL);
        assert!(board.snakes[0].is_alive());
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Left));
        assert!(!board.snakes[0].is_alive());
        assert!(is_empty(&board));
    }

    #[test]
    fn snake_dies_from_enemy_collision() {
        let mut board = create_board(data::BODY_COLLISION);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Right));
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());
    }

    #[test]
    fn two_snakes_in_head_to_head_collision() {
        let mut board = create_board(data::HEAD_TO_HEAD_BIG_AND_SMALL);
        advance_one_step(&mut board, &mut |i, _| {
            match i {
                0 => Action::Move(Movement::Down),
                1 => Action::Move(Movement::Right),
                _ => unreachable!(),
            }
        });
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());


        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL);
        advance_one_step(&mut board, &mut |i, _| {
            match i {
                0 => Action::Move(Movement::Right),
                1 => Action::Move(Movement::Left),
                _ => unreachable!(),
            }
        });
        assert!(!board.snakes[0].is_alive());
        assert!(!board.snakes[1].is_alive());


        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL_V2);
        advance_one_step(&mut board, &mut |i, _| {
            match i {
                0 => Action::Move(Movement::Down),
                1 => Action::Move(Movement::Up),
                _ => unreachable!(),
            }
        });
        assert!(!board.snakes[0].is_alive());
        assert!(!board.snakes[1].is_alive());

        let mut board = create_board(data::HEAD_TO_HEAD_CORRELATED_MCTS);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[0].health == 79);
        assert!(board.snakes[1].is_alive());
        assert!(board.snakes[1].health == 5);
        advance_one_step(&mut board, &mut |i, _| {
            match i {
                0 => Action::Move(Movement::Up),
                1 => Action::Move(Movement::Right),
                _ => unreachable!(),
            }
        });
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());


        let mut board = create_board(data::HEAD_TO_HEAD_OUT_OF_HEALTH);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[0].health == 1);
        assert!(board.snakes[1].is_alive());
        assert!(board.snakes[1].health == 61);
        advance_one_step(&mut board, &mut |i, _| {
            match i {
                0 => Action::Move(Movement::Down),
                1 => Action::Move(Movement::Left),
                _ => unreachable!(),
            }
        });
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());
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
        let mut board = create_board(data::FOLLOW_TAIL);
        let mut board_copy = board.clone();
        assert!(board.snakes[0].is_alive());
        // Make a full loop
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Left));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Up));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Right));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Right));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Down));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Down));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, &mut |_, _| Action::Move(Movement::Left));
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_eq!(board, board_copy);
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
