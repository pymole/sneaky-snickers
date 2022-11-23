use std::mem::{self, MaybeUninit};

use arrayvec::ArrayVec;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use crate::zobrist::body_direction;
use crate::game::{
    Board,
    MAX_SNAKE_COUNT,
    Point, WIDTH, HEIGHT, SIZE, Object, FOOD, BODY,
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

pub struct EngineSettings<'a, 'b> {
    // Can append elements to `board.food`, but must not mutate anything else.
    pub food_spawner: &'a mut dyn FnMut(&mut Board),

    // Can shrink `board.safe_zone`, but must not mutate anything else.
    pub safe_zone_shrinker: &'b mut dyn FnMut(&mut Board),
}

pub mod food_spawner {
    use super::*;
    use rand;

    pub fn dirty_store_empties_on_heads_if_there_is_no(board: &mut Board) -> [bool; MAX_SNAKE_COUNT] {
        // It's used in GameLog to save board.objects ordering.
        let mut restore_empty = [false; MAX_SNAKE_COUNT];
        for (i, snake) in board.snakes.iter().enumerate() {
            let head = snake.head();
            if board.objects.is_empty(head) {
                board.objects.set_body_on_empty(head);
                restore_empty[i] = true;
            }
        }
        restore_empty
    }

    pub fn dirty_restore_empties(board: &mut Board, restore_empties: [bool; MAX_SNAKE_COUNT]) {
        for (i, snake) in board.snakes.iter().enumerate() {
            if restore_empties[i] {
                board.objects.set_empty_on_body(snake.head());
            }
        }
    }

    fn spawn_one(rng: &mut impl rand::Rng, board: &mut Board) {
        // WARN: Dirty hack with heads. Set heads, generate food and restore empties again.
        // TODO: Check if there is no need to restore empties. "Make hack legal"
        
        let restore_empties = dirty_store_empties_on_heads_if_there_is_no(board);

        if let Some(empty_pos) = board.objects.get_empty_position(rng) {
            board.put_food(empty_pos);
        }

        dirty_restore_empties(board, restore_empties);
    }

    pub fn create_standard(board: &mut Board) {
        // For engine use only! It changes board.objects internal state
        let random = &mut rand::thread_rng();
        if board.foods.len() < 1 || random.gen_ratio(20, 100) {
            spawn_one(random, board);
        }
    }

    #[allow(dead_code)]
    pub fn noop(_: &mut Board) {
    }
}

pub mod safe_zone_shrinker {
    use super::*;

    pub fn shrink(board: &mut Board, side: Movement) {
        match side {
            Movement::Left => board.safe_zone.p0.x += 1,
            Movement::Down => board.safe_zone.p0.y += 1,
            Movement::Up => board.safe_zone.p1.y -= 1,
            Movement::Right => board.safe_zone.p1.x -= 1,
        }
    }

    // TODO: pub fn standard
    pub fn standard(board: &mut Board) {
        if board.turn == 0 || board.turn % 20 != 0 || board.safe_zone.empty(){
            return;
        }
        let side: Movement = rand::random();
        shrink(board, side);
    }

    #[allow(dead_code)]
    pub fn noop(_: &mut Board) {
    }
}

#[allow(dead_code)]
pub fn advance_one_step(board: &mut Board, actions: [usize; MAX_SNAKE_COUNT]) {
    let mut settings = EngineSettings {
        food_spawner: &mut food_spawner::noop,
        safe_zone_shrinker: &mut safe_zone_shrinker::noop,
    };

    advance_one_step_with_settings(board, &mut settings, actions)
}


pub fn advance_one_step_with_settings(
    board: &mut Board,
    engine_settings: &mut EngineSettings,
    actions: [usize; MAX_SNAKE_COUNT]
) {
    debug_assert!(!board.is_terminal(), "{}", board);

    board.zobrist_hash.xor_turn(board.turn);
    board.turn += 1;
    board.zobrist_hash.xor_turn(board.turn);

    debug_assert!(
        board.snakes.iter().all(|snake| snake.body.len() > 2)
    );

    // From https://docs.battlesnake.com/references/rules
    // 1. Each Battlesnake will have its chosen move applied:
    //     - A new body part is added to the board in the direction they moved.
    //     - Last body part (their tail) is removed from the board.
    //     - Health is reduced by 1.

    let alive_snakes: ArrayVec<usize, MAX_SNAKE_COUNT> = (0..board.snakes.len()).filter(|&i| board.snakes[i].is_alive()).collect();

    {
        for &snake_i in &alive_snakes {
            let movement = actions[snake_i];
            let snake = &mut board.snakes[snake_i];
            debug_assert!(snake.body.len() > 2);
            let old_head_position = snake.head();

            let mut new_head_position = old_head_position + Movement::from_usize(movement).to_direction();
            if new_head_position.x < 0 {
                new_head_position.x = WIDTH - 1;
            } else
            if new_head_position.y < 0 {
                new_head_position.y = HEIGHT - 1;
            } else
            if new_head_position.x == WIDTH {
                new_head_position.x = 0;
            } else
            if new_head_position.y == HEIGHT {
                new_head_position.y = 0;
            }

            snake.health -= 1;

            snake.body.push_front(new_head_position);

            // Move neck (first body part next to head)
            let new_neck_direction = body_direction(old_head_position, new_head_position);
            board.zobrist_hash.xor_body_direction(old_head_position, snake_i, new_neck_direction);

            // Remove old tail
            let old_tail = snake.body.pop_back().unwrap();
            let new_tail = snake.body[snake.body.len() - 1];
            debug_assert!(board.objects.is_body(old_tail));

            // TODO: Make zobrist hashing like this:
            // 0 [Still         ]                   - 3 body parts on the same spot
            // 1 [Still + Right ]                   - Head is out, body on the same spot
            // 2 [Right         ][Right         ]   - All body parts on unique spot
            
            let old_tail_direction = body_direction(old_tail, new_tail);
            board.zobrist_hash.xor_body_direction(old_tail, snake_i, old_tail_direction);

            if old_tail != new_tail {
                board.objects.set_empty_on_body(old_tail);
            }
            
            // The head will be set in a separate loop.
        }
    }

    let objects_under_head: [Object; MAX_SNAKE_COUNT] = {
        let mut objects_under_head: [MaybeUninit<Object>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
        
        for &i in &alive_snakes {
            let snake = &board.snakes[i];
            debug_assert!(board.contains(snake.head()));
            let object_under_head = board.objects.get(snake.head());
            objects_under_head[i] = MaybeUninit::new(object_under_head);
        }
        unsafe { mem::transmute(objects_under_head) }
    };

    // 2. Any Battlesnake that has found food will consume it:
    //     - Health reset set maximum.
    //     - Additional body part placed on top of current tail (this will extend their visible length by one on the
    //       next turn).
    //     - The food is removed from the board.

    // TODO: Try MaybeUninit
    let mut snake_ate_food = [false; MAX_SNAKE_COUNT];
    {
        let mut eaten_food = ArrayVec::<_, MAX_SNAKE_COUNT>::new();
        for &i in &alive_snakes {
            if objects_under_head[i] != FOOD {
                continue;
            }

            let snake = &mut board.snakes[i];
            let head = snake.head();

            snake.health = 100;

            let tail = *snake.body.back().unwrap();
            snake.body.push_back(tail);
            debug_assert!(board.objects.is_body(tail));
            eaten_food.push(head);
            snake_ate_food[i] = true;
        }
        for &food in eaten_food.iter() {
            if let Some(food_i) = board.foods.iter().position(|&x| x == food) {
                board.foods.swap_remove(food_i);
                // TODO: Проверить, что никто этот эмпти потом не будет изменять под другим типом
                board.objects.set_empty_on_food(food);
                board.zobrist_hash.xor_food(food);
            }
        }
    }

    // 3. Any new food spawning will be placed in empty objects on the board.
    // TODO: Fix bag when food generated under new head position.
    //  At this moment new head positions are empty
    {
        (engine_settings.food_spawner)(board);
    }


    //     - Collided with themselves
    //     - Collided with another Battlesnake
    //     - Collided head-to-head and lost

    {
        let mut died_snakes = ArrayVec::<usize, MAX_SNAKE_COUNT>::new();

        for &i in &alive_snakes {
            let object_under_head = objects_under_head[i];

            // Collided with themselves or another battlesnake.
            if object_under_head == BODY {
                died_snakes.push(i);
                continue;
            }

            let snake = &board.snakes[i];

            // Head-to-head.
            for &j in &alive_snakes {
                if i == j {
                    continue;
                }
                let other_snake = &board.snakes[j];

                if other_snake.is_alive()
                    && snake.body.len() <= other_snake.body.len()
                    && snake.head() == other_snake.head()
                {
                    died_snakes.push(i);
                    break;
                }
            }
        }

        for i in died_snakes {
            board.snakes[i].health = 0;
        }
    }

    // At last, aplly Battle Royale ruleset. Do in this order:
    // - Deal out-of-safe-zone damage
    // - Maybe shrink safe zone
    {
        for &i in &alive_snakes {
            if !snake_ate_food[i] {
                if board.is_hazard(board.snakes[i].head()) {
                    board.snakes[i].health -= 14;
                }
            }
        }

        (engine_settings.safe_zone_shrinker)(board);
    }

    // if cfg!(debug_assertions) {
        // println!("{:?} {}", board, board);
        // println!("{}", board);
    // }

    debug_assert!(
        board.snakes
            .iter()
            .filter(|snake| snake.is_alive())
            .all(|snake| board.foods.iter().all(|food| !snake.body.contains(&food)))
    );

    // Remove dead snakes from board.objects
    {
        for &i in &alive_snakes {
            let snake = &mut board.snakes[i];
            if !snake.is_alive() {
                snake.health = 0;

                // TODO: I don't understand why we have to put this empty.
                //  No food - it was eaten
                // NOTE: This assert is to check TODO above.
                if objects_under_head[i] != BODY {
                    debug_assert!(board.objects.is_empty(snake.head()));
                    // board.objects[snake.head()] = Object::Empty;
                }

                let mut front = snake.head();
                
                for j in 1..snake.body.len() - 1 {
                    let back = snake.body[j];
                    let body_part_direction = body_direction( back, front);
                    
                    board.objects.set_empty_on_body(back);
                    board.zobrist_hash.xor_body_direction(back, i, body_part_direction);

                    front = back;
                }

                // TODO: Check hashing behaviour. Hash must be consistent in
                //  multiple bodies on the cell problem.
                // Don't remove tail cell twice when on tail other part (when ate food or on first turn).
                let back = snake.body[snake.body.len() - 1];
                if back != front {
                    let body_part_direction = body_direction( back, front);                    
                    board.objects.set_empty_on_body(back);
                    board.zobrist_hash.xor_body_direction(back, i, body_part_direction);
                }
            }
        }

        // Restore alive heads, which were removed in previous loop in case of a head-to-head collision.
        for i in alive_snakes {
            if board.snakes[i].is_alive() {
                board.objects.set_body_on_empty(board.snakes[i].head());
            }
        }
    }

    if cfg!(debug_assertions) {
        let mut empties_on_map = 0;
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let pos = Point {x, y};
                if board.objects.is_empty(pos) {
                    empties_on_map += 1;
                    let i = board.objects.get(pos);
                    assert_eq!(board.objects.empties[i as usize], pos, "empties[{}] is not {:?}", i, pos);
                }
            }
        }
        assert_eq!(empties_on_map, board.objects.empties.len());
    }

    debug_assert_eq!(
        board.objects.empties_count(),
        SIZE as usize - board.foods.len() - board.snakes
            .iter()
            .filter(|snake| snake.is_alive())
            .map(|snake| {
                let mut body_occupied = snake.body.len();
                for i in 1..snake.body.len() {
                    if snake.body[i - 1] == snake.body[i] {
                        body_occupied = i;
                        break;
                    }
                }
                body_occupied
            })
            .sum::<usize>(),
        "{}",
        board,
    );
}

#[cfg(test)]
mod tests {
    use crate::test_data as data;
    use crate::test_utils::{test_transition, create_board, is_empty};

    use super::*;

    #[test]
    fn snake_moves_in_corect_direction() {
        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_UP,
            [Movement::Up as usize; MAX_SNAKE_COUNT]
        );

        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_LEFT,
            [Movement::Left as usize; MAX_SNAKE_COUNT]
        );

        test_transition(
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER,
            data::SINGLE_SHORT_SNAKE_IN_THE_CENTER_RIGHT,
            [Movement::Right as usize; MAX_SNAKE_COUNT]
        );

        {
            let mut board = create_board(data::SINGLE_SHORT_SNAKE_IN_THE_CENTER);
            advance_one_step(&mut board, [Movement::Down as usize; MAX_SNAKE_COUNT]);
            assert!(!board.snakes[0].is_alive(), "{:?}", &board.snakes[0]);
            assert!(is_empty(&board));
        }
    }

    #[test]
    fn health_depletes_by_one_on_each_move() {
        let mut board = create_board(data::SINGLE_SHORT_SNAKE_IN_THE_CENTER);
        assert_eq!(board.snakes[0].health, 100);
        advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
        assert_eq!(board.snakes[0].health, 99);
    }

    #[test]
    fn snake_eats_food() {
        let mut board = create_board(data::FOOD_IN_FRONT);
        assert_eq!(board.foods.len(), 3);
        assert_eq!(board.snakes[0].body.len(), 4);
        let food_point = board.foods[1];
        advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
        assert_eq!(board.snakes[0].body[0], food_point);
        assert_eq!(board.foods.len(), 2);
        assert_eq!(board.snakes[0].body.len(), 5);
        assert_eq!(board.snakes[0].body[3], board.snakes[0].body[4]);
        advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
        assert_eq!(board, create_board(data::FOOD_IN_FRONT_UP_UP));
    }

    #[test]
    fn snake_dies_out_of_bounds() {
        {
            let mut board = create_board(data::TOP_RIGHT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::TOP_RIGHT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, [Movement::Right as usize; MAX_SNAKE_COUNT]);
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::BOTTOM_LEFT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, [Movement::Left as usize; MAX_SNAKE_COUNT]);
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }

        {
            let mut board = create_board(data::BOTTOM_LEFT_CORNER);
            assert!(board.snakes[0].is_alive());
            advance_one_step(&mut board, [Movement::Down as usize; MAX_SNAKE_COUNT]);
            assert!(!board.snakes[0].is_alive());
            assert!(is_empty(&board));
        }
    }

    #[test]
    fn snake_dies_from_self_collision() {
        let mut board = create_board(data::STEP_ON_TAIL);
        assert!(board.snakes[0].is_alive());
        advance_one_step(&mut board, [Movement::Left as usize; MAX_SNAKE_COUNT]);
        assert!(!board.snakes[0].is_alive());
        assert!(is_empty(&board));
    }

    #[test]
    fn snake_dies_from_enemy_collision() {
        let mut board = create_board(data::BODY_COLLISION);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());
        advance_one_step(&mut board, [Movement::Right as usize; MAX_SNAKE_COUNT]);
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());
    }

    #[test]
    fn two_snakes_in_head_to_head_collision() {
        let mut board = create_board(data::HEAD_TO_HEAD_BIG_AND_SMALL);
        let mut actions = [0; MAX_SNAKE_COUNT];
        actions[0] = Movement::Down as usize;
        actions[1] = Movement::Right as usize;
        advance_one_step(&mut board, actions);
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());


        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL);
        let mut actions = [0; MAX_SNAKE_COUNT];
        actions[0] = Movement::Right as usize;
        actions[1] = Movement::Left as usize;
        advance_one_step(&mut board, actions);
        assert!(!board.snakes[0].is_alive());
        assert!(!board.snakes[1].is_alive());


        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL_V2);
        let mut actions = [0; MAX_SNAKE_COUNT];
        actions[0] = Movement::Down as usize;
        actions[1] = Movement::Up as usize;
        advance_one_step(&mut board, actions);
        assert!(!board.snakes[0].is_alive());
        assert!(!board.snakes[1].is_alive());

        let mut board = create_board(data::HEAD_TO_HEAD_CORRELATED_MCTS);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[0].health == 79);
        assert!(board.snakes[1].is_alive());
        assert!(board.snakes[1].health == 5);
        let mut actions = [0; MAX_SNAKE_COUNT];
        actions[0] = Movement::Up as usize;
        actions[1] = Movement::Right as usize;
        advance_one_step(&mut board, actions);
        assert!(!board.snakes[0].is_alive());
        assert!(board.snakes[1].is_alive());


        let mut board = create_board(data::HEAD_TO_HEAD_OUT_OF_HEALTH);
        assert!(board.snakes[0].is_alive());
        assert!(board.snakes[0].health == 1);
        assert!(board.snakes[1].is_alive());
        assert!(board.snakes[1].health == 61);
        let mut actions = [0; MAX_SNAKE_COUNT];
        actions[0] = Movement::Down as usize;
        actions[1] = Movement::Left as usize;
        advance_one_step(&mut board, actions);
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
        advance_one_step(&mut board, [Movement::Left as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Up as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Right as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Right as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Down as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Down as usize; MAX_SNAKE_COUNT]);
        board_copy.turn += 1;
        board_copy.snakes[0].health -= 1;
        assert_ne!(board, board_copy);
        advance_one_step(&mut board, [Movement::Left as usize; MAX_SNAKE_COUNT]);
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
