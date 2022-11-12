use crate::{game::{Board, random_point_inside_borders, Snake}};

pub fn generate_board() -> Board {
    // Generate board
    let head1 = random_point_inside_borders();
    let head2 = loop {
        let p = random_point_inside_borders();
        if p != head1 {
            break p
        }
    };

    let snakes = [
        Snake {
            health: 100,
            body: [head1, head1, head1].try_into().unwrap(),
        },
        Snake {
            health: 100,
            body: [head2, head2, head2].try_into().unwrap(),
        },
    ];

    // let foods = vec![];
    let hazard_start = random_point_inside_borders();
    // let hazards = vec![];

    let board = Board::new(
        0,
        None,
        Some(hazard_start),
        None,
        snakes,
    );

    board
}