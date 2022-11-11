use rocket::serde::json::serde_json;

use crate::{
    game::{
        Board,
        MAX_SNAKE_COUNT,
        Object,
        WIDTH,
        HEIGHT
    },
    api,
    engine::advance_one_step
};


pub fn create_board(api_str: &str) -> Board {
    let state = serde_json::from_str::<api::objects::State>(api_str).unwrap();
    Board::from_api(&state)
}

pub fn test_transition(state_before: &str, state_after: &str, actions: [usize; MAX_SNAKE_COUNT]) {
    let mut board_before = create_board(state_before);
    let board_after = create_board(state_after);
    advance_one_step(&mut board_before, actions);
    assert_eq!(board_before, board_after);
}

pub fn is_empty(board: &Board) -> bool {
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            if board.objects[(x, y)] != Object::Empty {
                return false;
            }
        }
    }

    return true;
}