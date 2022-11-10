use std::collections::HashSet;
use std::mem::{self, MaybeUninit};
use log::error;

use crate::api::objects::State;
use crate::engine::{EngineSettings, safe_zone_shrinker, advance_one_step_with_settings};
use crate::game::{Point, Board, Snake, MAX_SNAKE_COUNT};
use crate::zobrist::{body_direction, BodyDirections};

use bitvec::prelude::*;
use mongodb::sync::Client;
use serde::{Deserialize, Serialize, Serializer};

fn bit_vec_serialize<S>(x: &BitVec<u8, Msb0>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_bytes(x.as_raw_slice())
}

#[derive(Debug, Serialize, Deserialize)]
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


#[derive(Debug, Serialize, Deserialize)]
pub struct GameLog {
    // TODO: Use only inside selfplay, because Battlesnake API don't send
    #[serde(skip_serializing)]
    current_decomposition: ([Point; MAX_SNAKE_COUNT], HashSet<Point>),

    initial_board: BoardLog,
    
    #[serde(serialize_with = "bit_vec_serialize")]
    actions: BitVec<u8, Msb0>,
    #[serde(serialize_with = "bit_vec_serialize")]
    food: BitVec<u8, Msb0>,
    turns: usize,

    tags: Vec<String>,
}

impl GameLog {
    pub fn new_from_state(state: &State) -> GameLog {
        GameLog::new(
            Board::snake_api_to_snake_game(&state.board.snakes),
            &state.board.hazards,
            &state.board.food,
        )
    }

    pub fn new(
        snakes: [Snake; MAX_SNAKE_COUNT],
        hazards: &Vec<Point>,
        foods: &Vec<Point>,
    ) -> GameLog {
        let food = foods.iter().copied().collect();

        let heads: [Point; MAX_SNAKE_COUNT] = {
            let mut heads: [MaybeUninit<Point>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
            for i in 0..MAX_SNAKE_COUNT {
                heads[i] = MaybeUninit::new(snakes[i].head());
            }
            unsafe { mem::transmute(heads) }
        };

        let decomposition = (heads, food);

        GameLog {
            initial_board: BoardLog {
                food: foods.clone(),
                hazards: hazards.clone(),
                hazard_start: None,
                snakes,
            },
            current_decomposition: decomposition,
            actions: bitvec![u8, Msb0;],
            food: bitvec![u8, Msb0;],
            turns: 0,
            tags: Vec::new(),
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
            
            for i in 0..MAX_SNAKE_COUNT {
                let old_head = old_heads[i];
                let new_head = snakes[i].head();

                heads[i] = MaybeUninit::new(new_head);

                let direction = body_direction(old_head, new_head);
                if direction == BodyDirections::Still {
                    // Dead more than 1 turn ago (no move)
                    continue;
                }
                
                // 2 bits per move
                let action = direction as u8;
                let action_bits = action.view_bits::<Msb0>();
                self.actions.extend(&action_bits[action_bits.len() - 2 ..]);
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

            let food_x = food.x as u8;
            let x_bits = food_x.view_bits::<Msb0>();
            self.food.extend(&x_bits[x_bits.len() - 4 ..]);

            let food_y = food.y as u8;
            let y_bits = food_y.view_bits::<Msb0>();
            self.food.extend(&y_bits[y_bits.len() - 4 ..]);
        } else{
            self.food.push(false);
        }
        let food = spawned_foods.next();
        // Only one food spawned at the same time
        if food.is_some() {
            let food = food.unwrap();
            error!("EXTRA FOOD {} {}", food.x, food.y);
        }
        
        self.current_decomposition = (heads, foods);
    }

    pub fn set_hazard_start(&mut self, hazard_start: Point) {
        self.initial_board.hazard_start = Some(hazard_start);
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
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
    let mut boards = Vec::new();

    let mut board = Board::new(
        0,
        game_log.initial_board.food.clone(),
        game_log.initial_board.hazard_start,
        None,
        game_log.initial_board.snakes.clone(),
    );
    
    boards.push(board.clone());

    let food = &game_log.food;
    let mut f = 0;
    
    let mut log_food_spawner = |board: &mut Board| {
        let is_spawned = food[f];
        if is_spawned {
            let x: usize = food[f + 1..f + 5].load();
            let y: usize = food[f + 5..f + 9].load();
            
            let pos = Point { x: x as i32, y: y as i32 };
            board.put_food(pos);

            f += 9;
        } else {
            f += 1;
        }
    };

    let mut settings = EngineSettings {
        food_spawner: &mut log_food_spawner,
        safe_zone_shrinker: &mut safe_zone_shrinker::noop,
    };

    let actions_bits = &game_log.actions;
    let mut a = 0;

    for _ in 0..game_log.turns {
        let mut actions = [0; MAX_SNAKE_COUNT];
        for i in 0..board.snakes.len() {
            let action: usize = actions_bits[a..a + 2].load();
            actions[i] = action;

            a += 2;
        }

        advance_one_step_with_settings(&mut board, &mut settings, actions);
        boards.push(board.clone());
    }

    boards
}


#[cfg(test)]
mod tests {
    use rocket::serde::json::serde_json;

    use super::{GameLog, rewind};
    use crate::{test_data as data, game::{Board, Point}, api::{self, objects::Movement}, engine::{advance_one_step_with_settings, EngineSettings, safe_zone_shrinker}};
    
    fn create_board(api_str: &str) -> Board {
        let state = serde_json::from_str::<api::objects::State>(api_str).unwrap();
        Board::from_api(&state)
    }
    
    #[test]
    fn test_rewind() {
        let mut board = create_board(data::FOOD_HEAD_TO_HEAD_EQUAL);

        let mut game_log = GameLog::new(
            board.snakes.clone(),
            &Vec::new(),
            &board.foods,
        );

        game_log.set_hazard_start(board.hazard_start);

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
        game_log.add_turn_from_board(&board);
        boards.push(board.clone());
        // advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        // game_log.add_turn_from_board(&board);
        // boards.push(board.clone());
        // advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        // game_log.add_turn_from_board(&board);
        // boards.push(board.clone());
        // advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Up as usize]);
        // game_log.add_turn_from_board(&board);
        // boards.push(board.clone());
        // advance_one_step_with_settings(&mut board, &mut settings, [Movement::Up as usize, Movement::Left as usize]);
        // game_log.add_turn_from_board(&board);
        // boards.push(board.clone());

        println!("{}", board.turn);
        println!("{}", board.is_terminal);

        println!("{}", game_log.actions.clone().to_string());
        println!("{}", game_log.food.clone().to_string());

        let rewinded_boards = rewind(&game_log);
        
        assert_eq!(boards[0].snakes, rewinded_boards[0].snakes);
        assert_eq!(boards[0].foods, rewinded_boards[0].foods);
        assert_eq!(boards[0].hazard, rewinded_boards[0].hazard);
        assert_eq!(boards[0].hazard_start, rewinded_boards[0].hazard_start);
        assert_eq!(boards[0].is_terminal, rewinded_boards[0].is_terminal);

        assert_eq!(boards[1].turn, rewinded_boards[1].turn);
        assert_eq!(boards[1].snakes, rewinded_boards[1].snakes);
        assert_eq!(boards[1].foods, rewinded_boards[1].foods);
        assert_eq!(boards[1].hazard, rewinded_boards[1].hazard);
        assert_eq!(boards[1].hazard_start, rewinded_boards[1].hazard_start);
        assert_eq!(boards[1].is_terminal, rewinded_boards[1].is_terminal);
        assert_eq!(boards[1].objects, rewinded_boards[1].objects);
        assert_eq!(boards[1].zobrist_hash.get_value(), rewinded_boards[1].zobrist_hash.get_value());        

        assert_eq!(boards, rewinded_boards);
    }
}