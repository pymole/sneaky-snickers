
use arrayvec::ArrayVec;
use rand::{
    seq::SliceRandom,
    thread_rng,
    Rng,
};

use crate::game::{
    Board,
    Snake,
    MAX_SNAKE_COUNT,
    Point,
    WIDTH,
};

pub fn generate_board() -> Board {
    let snakes = make_snakes();

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

fn make_snakes() -> ArrayVec<Snake, MAX_SNAKE_COUNT> {
    // With fixed positions
    let rng = &mut thread_rng();

	// Create start 8 points
	let mn = 1;
    let md = (WIDTH - 1)/2;
    let mx = WIDTH - 2;

    let mut start_points = if rng.gen_bool(0.5) {
        let corner_points = [
            Point {x: mn, y: mn},
            Point {x: mn, y: mx},
            Point {x: mx, y: mn},
            Point {x: mx, y: mx},
        ];
        corner_points
    } else {
        let cardinal_points = [
            Point {x: mn, y: md},
            Point {x: md, y: mn},
            Point {x: md, y: mx},
            Point {x: mx, y: md},
        ];
        cardinal_points
    };
	start_points.shuffle(rng);
	
	// Assign to snakes in order given
    let mut snakes = ArrayVec::new();
    for i in 0..MAX_SNAKE_COUNT {
        let p = start_points[i];
        let snake = Snake {
            health: 100,
            body: [p, p, p].try_into().unwrap(),
        };
        snakes.push(snake);
    }
    snakes
}

fn make_food() {
    
}