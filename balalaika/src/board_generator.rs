use std::collections::HashSet;

use crate::{game::{Board, random_point_inside_borders, Snake}};

pub fn generate_board() -> Board {
    let mut heads = HashSet::new();
    let head = random_point_inside_borders();
    heads.insert(head);
    while heads.len() < 4 {
        let head = random_point_inside_borders();
        if !heads.contains(&head) {
            heads.insert(head);
        }
    }

    let snakes = heads
        .into_iter()
        .map(|head| Snake {
            health: 100,
            body: [head, head, head].try_into().unwrap(),
        })
        .collect();

    // let foods = vec![];
    // let hazards = vec![];

    let board = Board::new(
        0,
        None,
        None,
        snakes,
    );

    board
}