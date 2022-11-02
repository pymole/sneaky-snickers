use std::collections::HashMap;

use crate::game::Point;
use crate::zobrist::body_direction;
use crate::api::objects::State;
use bitvec::prelude::*;



pub struct GameLog {
    state_decomposition: (HashMap<String, Point>,),
    actions: BitVec<u8, Msb0>,
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
            state_decomposition: decompose_state(state),
            actions: bitvec![u8, Msb0;],
        }
    }

    pub fn add_state(&mut self, state: &State) {
        // Дохуя ходов
        // Это означает, что нужно будет заполнять динмически
        // По ходу игры количество игроков будет сокращаться, поэтому на один ход не статичное количество битов
        // 
        // 2 bits per direction on 11x11 board

        let state_decomposition = decompose_state(state);

        let old_heads = &self.state_decomposition.0;
        
        for (snake_id, new_head) in &state_decomposition.0 {
            if old_heads.contains_key(snake_id) {
                let old_head = &old_heads[snake_id];
                let action = body_direction(*old_head, *new_head) as usize;
                
                let action_bits = &action.view_bits::<Lsb0>()[..2];
                self.actions.extend(action_bits);
            }
        }

        self.state_decomposition = state_decomposition;
    }

    pub fn to_string(&self) -> String {
        self.actions.to_string()
    }
}

