use std::collections::HashMap;

use crate::game::Point;
use crate::zobrist::body_direction;
use crate::api::objects::State;

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
    current_state_decomposition: (HashMap<String, Point>,),
    
    initial_state: State,
    #[serde(serialize_with = "bit_vec_serialize")]
    actions: BitVec<u8, Lsb0>,
    turns: usize,
}

fn decompose_state(state: &State) -> (HashMap<String, Point>,) {
    let mut heads: HashMap<String, Point> = HashMap::new();
    for snake in state.board.snakes.iter() {
        heads.insert(snake.id.clone(), snake.head);
    }
    
    (heads, )
}


impl GameLog {
    pub fn new(state: &State) -> GameLog {
        GameLog {
            initial_state: state.clone(),
            current_state_decomposition: decompose_state(state),
            actions: bitvec![u8, Lsb0;],
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
                
                let action_bits = &action.view_bits::<Lsb0>()[..2];
                self.actions.extend(action_bits);
            }
        }

        self.current_state_decomposition = state_decomposition;
    }
}

