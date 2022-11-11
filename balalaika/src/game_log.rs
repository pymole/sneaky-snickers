use std::collections::HashSet;
use std::mem::{self, MaybeUninit};
use log::error;

use crate::api::objects::State;
use crate::engine::{EngineSettings, safe_zone_shrinker, advance_one_step_with_settings};
use crate::game::{Point, Board, Snake, MAX_SNAKE_COUNT, WIDTH, HEIGHT};
use crate::zobrist::{body_direction, BodyDirections};

use bitvec::prelude::*;
use mongodb::sync::Client;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameLog {
    initial_board: BoardLog,
    #[serde(with = "serde_bytes")]
    actions: Vec<u8>,
    #[serde(with = "serde_bytes")]
    food: Vec<u8>,
    turns: usize,
    tags: Vec<String>,
}

impl GameLog {
    pub fn get_foods(&self) -> Vec<Option<Point>> {
        let mut foods = Vec::new();
        let foods_bits: BitVec<_, Msb0> = BitVec::from_vec(self.food.clone());

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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BoardLog {
    food: Vec<Point>,
    hazards: Vec<Point>,
    hazard_start: Option<Point>,
    snakes: [Snake; MAX_SNAKE_COUNT],
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
    current_decomposition: ([Point; MAX_SNAKE_COUNT], HashSet<Point>),
    initial_board: BoardLog,
    actions: BitVec<u8, Msb0>,
    food: BitVec<u8, Msb0>,
    turns: usize,
    tags: Vec<String>,
}

impl GameLogBuilder {
    pub fn new_from_state(state: &State) -> GameLogBuilder {
        GameLogBuilder::new(
            Board::snake_api_to_snake_game(&state.board.snakes),
            &state.board.hazards,
            &state.board.food,
        )
    }

    pub fn new(
        snakes: [Snake; MAX_SNAKE_COUNT],
        hazards: &Vec<Point>,
        foods: &Vec<Point>,
    ) -> GameLogBuilder {
        let food = foods.iter().copied().collect();

        let heads: [Point; MAX_SNAKE_COUNT] = {
            let mut heads: [MaybeUninit<Point>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
            for i in 0..MAX_SNAKE_COUNT {
                heads[i] = MaybeUninit::new(snakes[i].head());
            }
            unsafe { mem::transmute(heads) }
        };

        let current_decomposition = (heads, food);

        let initial_board = BoardLog {
            food: foods.clone(),
            hazards: hazards.clone(),
            hazard_start: None,
            snakes,
        };

        GameLogBuilder {
            turns: 0,
            food: Default::default(),
            actions: Default::default(),
            tags: Default::default(),
            initial_board,
            current_decomposition,
        }
    }

    pub fn add_turn_from_state(&mut self, state: &State) {
        self.add_turn(
            &Board::snake_api_to_snake_game(&state.board.snakes),
            &state.board.food,
        )
    }

    pub fn add_turn_from_board(&mut self, board: &Board) {
        self.add_turn(
            &board.snakes,
            &board.foods,
        )
    }

    pub fn add_turn(
        &mut self,
        snakes: &[Snake; MAX_SNAKE_COUNT],
        foods: &Vec<Point>
    ) {
        self.turns += 1;

        // Actions
        
        let heads: [Point; MAX_SNAKE_COUNT] = {
            let mut heads: [MaybeUninit<Point>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };

            let old_heads = &self.current_decomposition.0;

            // 2 bits per move
            self.actions.reserve(2 * MAX_SNAKE_COUNT);
            
            for i in 0..MAX_SNAKE_COUNT {
                let old_head = old_heads[i];
                let new_head = snakes[i].head();

                let direction = body_direction(old_head, new_head);
                if direction == BodyDirections::Still {
                    // Dead more than 1 turn ago (no move)
                    continue;
                }

                heads[i] = MaybeUninit::new(new_head);

                println!("{:?}", direction as usize);

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
        
        self.current_decomposition = (heads, foods);
    }

    pub fn set_hazard_start(&mut self, hazard_start: Point) {
        self.initial_board.hazard_start = Some(hazard_start);
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn finalize(&self) -> GameLog {
        let mut food = self.food.clone();
        food.shrink_to_fit();

        let mut actions = self.actions.clone();
        actions.shrink_to_fit();        

        GameLog {
            initial_board: self.initial_board.clone(),
            actions: actions.into_vec(),
            food: food.into_vec(),
            turns: self.turns,
            tags: self.tags.clone(),
        }
    }
}

pub fn save_game_log(client: &Client, game_log: &GameLog) {
    let db = client.default_database().expect("Default database must be provided");
    let games = db.collection::<GameLog>("games");
    let insert_res = games.insert_one(game_log, None);
    if let Err(err) = insert_res {
        error!("Failed to insert game log: {}", err);
    }
}

pub fn rewind(game_log: &GameLog) -> Vec<Board> {
    println!("REWIND");
    let mut boards = Vec::new();

    let mut board = Board::new(
        0,
        game_log.initial_board.food.clone(),
        game_log.initial_board.hazard_start,
        None,
        game_log.initial_board.snakes.clone(),
    );
    
    boards.push(board.clone());

    let foods = game_log.get_foods();
    
    let mut log_food_spawner = |board: &mut Board| {
        if let Some(food) = foods[board.turn as usize - 1] {
            board.put_food(food);
        }
    };

    let mut settings = EngineSettings {
        food_spawner: &mut log_food_spawner,
        safe_zone_shrinker: &mut safe_zone_shrinker::noop,
    };

    let mut actions_bits = game_log.actions.as_bits::<Msb0>().chunks_exact(2);

    for _ in 0..game_log.turns {
        let mut actions = [0; MAX_SNAKE_COUNT];
        for i in 0..board.snakes.len() {
            let d = actions_bits.next().expect("Corrupted actions").load_be();
            println!("{:?}", d);
            actions[i] = d
        }

        advance_one_step_with_settings(&mut board, &mut settings, actions);
        boards.push(board.clone());
    }

    boards
}


#[cfg(test)]
mod tests {
    use super::{GameLogBuilder, rewind};
    use crate::test_utils::create_board;
    use crate::{
        test_data as data,
        game::{Board, Point},
        api::objects::Movement,
        engine::{advance_one_step_with_settings, EngineSettings, safe_zone_shrinker}
    };
    
    #[test]
    fn test_rewind() {
        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL);

        let mut game_log_builder = GameLogBuilder::new(
            board.snakes.clone(),
            &Vec::new(),
            &board.foods,
        );

        game_log_builder.set_hazard_start(board.hazard_start);

        let mut spawn_food_at_2_and_4_turn = |board: &mut Board| {
            if board.turn == 2 {
                board.put_food(Point {x: 0, y: 0});
            }
            else
            if board.turn == 4 {
                board.put_food(Point {x: 10, y: 10});
            }
        };

        let mut settings = EngineSettings {
            food_spawner: &mut spawn_food_at_2_and_4_turn,
            safe_zone_shrinker: &mut safe_zone_shrinker::noop,
        };

        let mut boards = Vec::new();
        boards.push(board.clone());

        advance_one_step_with_settings(&mut board, &mut settings, [Movement::Right as usize, Movement::Up as usize]);
        game_log_builder.add_turn_from_board(&board);
        boards.push(board.clone());
        advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        game_log_builder.add_turn_from_board(&board);
        boards.push(board.clone());
        advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        game_log_builder.add_turn_from_board(&board);
        boards.push(board.clone());
        advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        game_log_builder.add_turn_from_board(&board);
        boards.push(board.clone());
        advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Left as usize]);
        game_log_builder.add_turn_from_board(&board);
        boards.push(board.clone());

        let game_log = game_log_builder.finalize();

        let rewinded_boards = rewind(&game_log);

        assert_eq!(boards, rewinded_boards);
    }
}