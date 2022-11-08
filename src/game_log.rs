use std::collections::{HashMap, HashSet};

use crate::game::Point;
use crate::zobrist::body_direction;
use crate::api::objects::State;

use bitvec::prelude::*;
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
    snakes: Vec<SnakeLog>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnakeLog {
    pub id: String,
    pub health: i32,
    pub body: Vec<Point>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GameLog {
    #[serde(skip)]
    current_state_decomposition: (HashMap<String, Point>, HashSet<Point>),

    timeout: i32,
    
    initial_board: BoardLog,
    
    #[serde(serialize_with = "bit_vec_serialize")]
    actions: BitVec<u8, Msb0>,
    #[serde(serialize_with = "bit_vec_serialize")]
    food: BitVec<u8, Msb0>,
    turns: usize,
}

fn decompose_state(state: &State) -> (HashMap<String, Point>, HashSet<Point>) {
    let mut heads: HashMap<String, Point> = HashMap::new();
    for snake in state.board.snakes.iter() {
        heads.insert(snake.id.clone(), snake.head);
    }

    let mut foods = HashSet::new();
    for &food in state.board.food.iter() {
        foods.insert(food);
    }

    (heads, foods)
}


impl GameLog {
    pub fn new(state: &State) -> GameLog {
        let snakes: Vec<SnakeLog> = state.board.snakes
            .iter()
            .map(|snake| SnakeLog {
                id: snake.id.clone(),
                body: snake.body.clone(),
                health: snake.health,
            })
            .collect();

        GameLog {
            timeout: state.game.timeout,
            initial_board: BoardLog {
                food: state.board.food.clone(),
                hazards: state.board.hazards.clone(),
                snakes,
            },
            current_state_decomposition: decompose_state(state),
            actions: bitvec![u8, Msb0;],
            food: bitvec![u8, Msb0;],
            turns: 0,
        }
    }

    pub fn add_state(&mut self, state: &State) {
        self.turns += 1;

        let state_decomposition = decompose_state(state);

        let old_heads = &self.current_state_decomposition.0;
        let new_heads = &state_decomposition.0;
        
        for (snake_id, new_head) in new_heads {
            if old_heads.contains_key(snake_id) {
                let old_head = &old_heads[snake_id];
                let action = body_direction(*old_head, *new_head) as u8;
                
                // 2 bits per move
                let action_bits = action.view_bits::<Msb0>();
                self.actions.extend(&action_bits[action_bits.len() - 2 ..]);
            }
        }
        
        let old_foods = &self.current_state_decomposition.1;
        let new_foods = &state_decomposition.1;
        let mut spawned_foods = new_foods.difference(old_foods);
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

        // Only one food spawned at the same time
        assert!(spawned_foods.next().is_none());
        
        self.current_state_decomposition = state_decomposition;
    }
}

