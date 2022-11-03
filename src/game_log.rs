use std::collections::{HashMap, HashSet};

use crate::game::Point;
use crate::zobrist::body_direction;
use crate::api::objects::{State, Game, Board};

use bitvec::prelude::*;
use serde::{Deserialize, Serialize, Serializer};


fn bit_vec_serialize<S>(x: &BitVec<u8, Lsb0>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_bytes(x.as_raw_slice())
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GameLog {
    #[serde(skip)]
    current_state_decomposition: (HashMap<String, Point>, HashSet<Point>),
    
    game: Game,
    initial_board: Board,
    #[serde(serialize_with = "bit_vec_serialize")]
    actions: BitVec<u8, Lsb0>,
    #[serde(serialize_with = "bit_vec_serialize")]
    food: BitVec<u8, Lsb0>,
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
        GameLog {
            initial_board: state.board.clone(),
            game: state.game.clone(),
            current_state_decomposition: decompose_state(state),
            actions: bitvec![u8, Lsb0;],
            food: bitvec![u8, Lsb0;],
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
                let action = body_direction(*old_head, *new_head) as usize;
                
                // 2 bits per move
                let action_bits = &action.view_bits::<Lsb0>()[..2];
                self.actions.extend(action_bits);
            }
        }

        let old_foods = &self.current_state_decomposition.1;
        let new_foods = &state_decomposition.1;
        let spawned_foods = new_foods.difference(old_foods).into_iter();

        for food in spawned_foods {
            // 4 bits per coord, 8 bits
            self.food.extend(&(food.x as usize).view_bits::<Lsb0>()[..4]);
            self.food.extend(&(food.y as usize).view_bits::<Lsb0>()[..4]);
        }
        
        self.current_state_decomposition = state_decomposition;
    }
}

