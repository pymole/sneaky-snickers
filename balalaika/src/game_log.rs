use std::collections::HashSet;
use std::mem::{self, MaybeUninit};
use arrayvec::ArrayVec;
use mongodb::bson::{doc, Bson};
use mongodb::error::Error;
use mongodb::results::InsertOneResult;

use crate::api::objects::{State, Movement};
use crate::engine::safe_zone_shrinker::shrink;
use crate::engine::{EngineSettings, advance_one_step_with_settings};
use crate::game::{Point, Board, Snake, MAX_SNAKE_COUNT, WIDTH, HEIGHT, Rectangle};
use crate::zobrist::{body_direction, BodyDirections};

use bitvec::prelude::*;
use mongodb::sync::Client;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GameLog {
    initial_board: BoardLog,
    #[serde(with = "serde_bytes")]
    pub actions: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub food: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub shrinks: Vec<u8>,
    pub turns: usize,
    tags: Vec<String>,
}

impl GameLog {
    pub fn get_foods(&self) -> Vec<Option<Point>> {
        let mut foods = Vec::new();
        let foods_bits: BitVec<u8, Msb0> = BitVec::from_vec(self.food.clone());

        let mut i = 0;
        for _ in 0..self.turns {
            let is_spawned = foods_bits[i];
            if is_spawned {
                let x: u32 = foods_bits[i + 1 .. i + 5].load_be();
                let y: u32 = foods_bits[i + 5 .. i + 9].load_be();

                assert!(x < WIDTH as u32, "{}", x);
                assert!(y < HEIGHT as u32, "{}", y);
                
                foods.push(Some(Point {x: x as i32, y: y as i32}));

                i += 9;
            } else {
                foods.push(None);
                i += 1;
            }
        }

        foods
    }

    pub fn get_shrinks(&self) -> Vec<Option<Movement>> {
        let mut shrinks = Vec::new();
        let shrinks_bits: BitVec<u8, Msb0> = BitVec::from_vec(self.shrinks.clone());

        let mut i = 0;
        for _ in 0..self.turns {
            let shrunk = shrinks_bits[i];
            if shrunk {
                let side: u32 = shrinks_bits[i + 1 .. i + 3].load_be();
                
                shrinks.push(Some(Movement::from_usize(side as usize)));

                i += 3;
            } else {
                shrinks.push(None);
                i += 1;
            }
        }

        shrinks
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
struct BoardLog {
    food: Vec<Point>,
    safe_zone: Rectangle,
    snakes: ArrayVec<Snake, MAX_SNAKE_COUNT>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnakeLog {
    pub health: i32,
    pub body: Vec<Point>,
}


#[derive(Debug)]
pub struct GameLogBuilder {
    // TODO: Use only inside selfplay, because Battlesnake API don't send
    // dead snakes and we can't figure out where they moved last turn.
    current_decomposition: ([Point; MAX_SNAKE_COUNT], HashSet<Point>, Rectangle),
    initial_board: BoardLog,
    actions: BitVec<u8, Msb0>,
    food: BitVec<u8, Msb0>,
    shrinks: BitVec<u8, Msb0>,
    turns: usize,
    tags: Vec<String>,
}

impl GameLogBuilder {
    pub fn new_from_state(state: &State) -> GameLogBuilder {
        GameLogBuilder::new(
            Board::snake_api_to_snake_game(&state.board.snakes),
            Board::calculate_safe_zone(&state.board.hazards),
            &state.board.food,
        )
    }

    pub fn new(
        snakes: ArrayVec<Snake, MAX_SNAKE_COUNT>,
        safe_zone: Rectangle,
        foods: &Vec<Point>,
    ) -> GameLogBuilder {
        let food = foods.iter().copied().collect();

        let heads: [Point; MAX_SNAKE_COUNT] = {
            let mut heads: [MaybeUninit<Point>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
            for (i, snake) in snakes.iter().enumerate() {
                heads[i] = MaybeUninit::new(snake.head());
            }
            unsafe { mem::transmute(heads) }
        };

        let current_decomposition = (heads, food, safe_zone);

        let initial_board = BoardLog {
            food: foods.clone(),
            safe_zone,
            snakes,
        };

        GameLogBuilder {
            turns: 0,
            food: Default::default(),
            actions: Default::default(),
            tags: Default::default(),
            shrinks: Default::default(),
            initial_board,
            current_decomposition,
        }
    }

    pub fn add_turn_from_state(&mut self, state: &State) {
        self.add_turn(
            &Board::snake_api_to_snake_game(&state.board.snakes),
            &state.board.food,
            Board::calculate_safe_zone(&state.board.hazards),
        )
    }

    pub fn add_turn_from_board(&mut self, board: &Board) {
        self.add_turn(
            &board.snakes,
            &board.foods,
            board.safe_zone,
        )
    }

    pub fn add_turn(
        &mut self,
        snakes: &ArrayVec<Snake, MAX_SNAKE_COUNT>,
        foods: &Vec<Point>,
        safe_zone: Rectangle,
    ) {
        self.turns += 1;

        // Actions
        
        let heads: [Point; MAX_SNAKE_COUNT] = {
            let mut heads: [MaybeUninit<Point>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };

            let old_heads = &self.current_decomposition.0;

            // 2 bits per move
            for i in 0..snakes.len() {
                let old_head = old_heads[i];
                let new_head = snakes[i].head();

                let direction = body_direction(old_head, new_head);
                if direction == BodyDirections::Still {
                    // Dead more than 1 turn ago (no move)
                    heads[i] = MaybeUninit::new(old_head);
                    continue;
                }

                // println!("{:?} {}", direction, snakes.iter().filter(|s| s.is_alive()).count());

                heads[i] = MaybeUninit::new(new_head);

                self.actions.reserve(2);
                let len = self.actions.len();
                unsafe { self.actions.set_len(len + 2) };
                
                self.actions[len .. len + 2].store_be(direction as usize);
            }
            unsafe { mem::transmute(heads) }
        };

        // Food spawn
        let foods: HashSet<_> = foods.iter().copied().collect();
        let old_foods = &self.current_decomposition.1;
        let mut spawned_foods = foods.difference(old_foods);

        // 1 bit - spawned or not.
        // If spawned then 4 bits per coord (on board 11 x 11).
        if let Some(food) = spawned_foods.next() {
            self.food.push(true);

            let mut len = self.food.len();
            self.food.reserve(8);
            unsafe { self.food.set_len(len + 8) };
            self.food[len .. len + 4].store_be(food.x as u32);
            len += 4;
            self.food[len .. len + 4].store_be(food.y as u32);
        } else{
            self.food.push(false);
        }

        // Only one food spawned at the same time
        assert!(spawned_foods.next().is_none());

        // Safe zone shrinking
        let old_safe_zone = self.current_decomposition.2;
        let side = if old_safe_zone.p0.x != safe_zone.p0.x {
            Some(Movement::Left)
        } else if old_safe_zone.p0.y != safe_zone.p0.y {
            Some(Movement::Down)
        } else if old_safe_zone.p1.x != safe_zone.p1.x {
            Some(Movement::Right)
        } else if old_safe_zone.p1.y != safe_zone.p1.y {
            Some(Movement::Up)
        } else {
            None
        };

        if let Some(side) = side {
            self.shrinks.push(true);

            let len = self.shrinks.len();
            self.shrinks.reserve(2);
            unsafe { self.shrinks.set_len(len + 2) };
            self.shrinks[len .. len + 2].store_be(side as u32);
        } else {
            self.shrinks.push(false);
        }
        
        self.current_decomposition = (heads, foods, safe_zone);
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn finalize(&self) -> GameLog {
        let mut food = self.food.clone();
        food.shrink_to_fit();

        let mut actions = self.actions.clone();
        actions.shrink_to_fit();

        let mut shrinks = self.shrinks.clone();
        shrinks.shrink_to_fit();

        GameLog {
            initial_board: self.initial_board.clone(),
            actions: actions.into_vec(),
            food: food.into_vec(),
            shrinks: shrinks.into_vec(),
            turns: self.turns,
            tags: self.tags.clone(),
        }
    }
}

pub fn save_game_log(client: &Client, game_log: &GameLog) -> Result<InsertOneResult, Error> {
    let db = client.default_database().expect("Default database must be provided");
    let games = db.collection::<GameLog>("games");
    let insert_res = games.insert_one(game_log, None);

    insert_res

    // TODO: Return result, handle error outside
    // if let Err(err) = insert_res {
    //     error!("Failed to insert game log: {}", err);
    // }
}

pub fn load_game_log(client: &Client, id: Bson) -> Result<Option<GameLog>, Error> {
    let db = client.default_database().expect("Default database must be provided");
    let games = db.collection::<GameLog>("games");
    let game_log = games.find_one(doc! {"_id": id }, None);

    game_log
}


pub fn rewind(game_log: &GameLog) -> (Vec<[usize; MAX_SNAKE_COUNT]>, Vec<Board>) {
    // println!("REWIND");
    assert!(!game_log.initial_board.food.is_empty());

    let mut boards = Vec::new();

    let mut board = Board::new(
        0,
        Some(game_log.initial_board.food.clone()),
        Some(game_log.initial_board.safe_zone),
        game_log.initial_board.snakes.clone(),
    );
    
    boards.push(board.clone());

    let foods = game_log.get_foods();
    
    let mut log_food_spawner = |board: &mut Board| {
        if let Some(food) = foods[board.turn as usize - 1] {
            board.put_food(food);
        }
    };

    let shrinks = game_log.get_shrinks();

    let mut log_safe_zone_shrinker = |board: &mut Board| {
        if let Some(side) = shrinks[board.turn as usize - 1] {
            shrink(board, side);
        }
    };

    let mut settings = EngineSettings {
        food_spawner: &mut log_food_spawner,
        safe_zone_shrinker: &mut log_safe_zone_shrinker,
    };

    let mut game_actions = Vec::new();
    let mut actions_bits = game_log.actions.as_bits::<Msb0>().chunks_exact(2);

    for _ in 0..game_log.turns {
        let mut actions = [0; MAX_SNAKE_COUNT];
        for i in 0..board.snakes.len() {
            if !board.snakes[i].is_alive() {
                continue;
            }
            actions[i] = actions_bits.next().expect("Corrupted actions").load_be();
        }

        // println!("{:?}", actions);

        game_actions.push(actions);

        advance_one_step_with_settings(&mut board, &mut settings, actions);
        boards.push(board.clone());
    }

    assert!(board.is_terminal(), "\nBoard is not terminal\n {} {:?}", board, board);

    (game_actions, boards)
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::collections::hash_map::RandomState;

    use mongodb::sync::Client;
    use rand::thread_rng;
    use pretty_assertions::{assert_eq};

    use super::{GameLogBuilder, rewind, GameLog};
    use crate::board_generator::generate_board;
    use crate::engine::food_spawner;
    use crate::game_log::{save_game_log, load_game_log};
    use crate::mcts::utils::get_random_actions_from_masks;
    use crate::{
        game::Point,
        engine::{advance_one_step_with_settings, EngineSettings, safe_zone_shrinker}
    };
    
    #[test]
    fn test_rewind_random_play_integrational() {
        // NOTE: Run MongoDB: docker compose up

        let client = Client::with_uri_str("mongodb://battlesnake:battlesnake@localhost:27017/battlesnake?authSource=admin").unwrap();
        let db = client.default_database().expect("Provide default database");
        db.collection::<GameLog>("rewind_test").drop(None);
        db.create_collection("rewind_test", None).expect("Collection rewind_test isn't created");

        let mut random = thread_rng();
        for _ in 0..100 {
            println!("OK");
            let mut board = generate_board();
            let mut game_log_builder = GameLogBuilder::new(
                board.snakes.clone(),
                board.safe_zone,
                &board.foods,
            );
            

            let mut engine_settings = EngineSettings {
                food_spawner: &mut food_spawner::create_standard,
                safe_zone_shrinker: &mut safe_zone_shrinker::standard,
            };

            let mut actual_actions =  Vec::new();
            let mut actual_boards =  Vec::new();
   
            actual_boards.push(board.clone());

            while !board.is_terminal() {
                let actions = get_random_actions_from_masks(&mut random, &board);
                actual_actions.push(actions);
                advance_one_step_with_settings(&mut board, &mut engine_settings, actions);
                game_log_builder.add_turn_from_board(&board);
                // println!("{}", board);
                actual_boards.push(board.clone());
            }

            let game_log_constructed = game_log_builder.finalize();

            let insert_res = save_game_log(&client, &game_log_constructed);

            let load_res = load_game_log(
                &client,
                insert_res.expect("Failed to write game").inserted_id
            );
            let game_log_loaded = load_res.expect("Failed to load").unwrap();

            assert_eq!(game_log_constructed, game_log_loaded);

            let (actions, boards) = rewind(&game_log_loaded);
            assert_eq!(actions.len(), boards.len() - 1, "Actions and boards count don't match");

            assert_eq!(actual_actions.len(), actions.len(), "Different actions count");
            for i in 0..actual_actions.len() {
                assert_eq!(actual_actions[i], actions[i], "Different action on turn {}", i);
            }

            assert_eq!(actual_boards.len(), boards.len(), "Different positions count");
            for i in 0..actual_boards.len() {
                assert_eq!(actual_boards[i].safe_zone, boards[i].safe_zone, "{}", board);
                assert_eq!(actual_boards[i].snakes, boards[i].snakes, "{}", board);
                assert_eq!(actual_boards[i].foods, boards[i].foods, "{}", board);
                assert_eq!(actual_boards[i].zobrist_hash, boards[i].zobrist_hash, "{}", board);
                // TODO: On terminal board empties order is changed.
                //  That's strange but have no impact because:
                //  1. empties used only for random pick.
                //  2. it's inside terminal node.
                //  Remove HashSet and see error.
                // map is also corrupted

                assert_eq!(
                    HashSet::<Point, RandomState>::from_iter(actual_boards[i].objects.empties.iter().copied()),
                    HashSet::from_iter(boards[i].objects.empties.iter().copied()),
                    "{}\n\n{:?}", board, board
                );
                // assert_eq!(actual_boards[i].objects.empties, boards[i].objects.empties, "{}", board);
                // assert_eq!(actual_boards[i].objects.map, boards[i].objects.map, "{}", board);
            }
        }

        db.collection::<GameLog>("rewind_test").drop(None).expect("Error on rewind_test collection delete");
    }

    #[test]
    fn test_rewind_main_collection() {
        let client = Client::with_uri_str("mongodb://battlesnake:battlesnake@localhost:27017/battlesnake?authSource=admin").unwrap();
        let db = client.default_database().expect("Default database must be provided");
        let games = db.collection::<GameLog>("games");
        let game_log = games.find_one(None, None).expect("Loading error").expect("No games in main DB");
        let _ = rewind(&game_log);
    }
}