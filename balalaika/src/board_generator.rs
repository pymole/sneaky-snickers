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
    GridPoint,
    WIDTH, HEIGHT, CENTER,
};

pub fn generate_board() -> Board {
    let snakes = make_snakes();
    let foods = make_food(&snakes);
    let board = Board::new(
        0,
        Some(foods),
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
            GridPoint {x: mn, y: mn},
            GridPoint {x: mn, y: mx},
            GridPoint {x: mx, y: mn},
            GridPoint {x: mx, y: mx},
        ];
        corner_points
    } else {
        let cardinal_points = [
            GridPoint {x: mn, y: md},
            GridPoint {x: md, y: mn},
            GridPoint {x: md, y: mx},
            GridPoint {x: mx, y: md},
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

fn make_food(snakes: &ArrayVec<Snake, MAX_SNAKE_COUNT>) -> Vec<GridPoint> {
    let rng = &mut thread_rng();

	let center = GridPoint {
        x: (WIDTH - 1) / 2,
        y: (HEIGHT - 1) / 2,
    };

    let mut foods = Vec::new();
	
    // Place 1 food within exactly 2 moves of each snake, but never towards the center or in a corner
    for snake in snakes {
        let head = snake.head();
        let possible_food_locations = [
            GridPoint {x: head.x - 1, y: head.y - 1},
            GridPoint {x: head.x - 1, y: head.y + 1},
            GridPoint {x: head.x + 1, y: head.y - 1},
            GridPoint {x: head.x + 1, y: head.y + 1},
        ];

        // Remove any invalid/unwanted positions
        let mut available_food_locations = Vec::new();
        for p in possible_food_locations {
            // Food must be further than snake from center on at least one axis
            if !(
                p.x < head.x && head.x < center.x ||
                center.x < head.x && head.x < p.x ||
                p.y < head.y && head.y < center.y ||
                center.y < head.y && head.y < p.y
            ) {
                continue;
            }

            // Don't spawn food in corners
            if (p.x == 0 || p.x == (WIDTH - 1)) && (p.y == 0 || p.y == (HEIGHT - 1)) {
                continue
            }

            available_food_locations.push(p);
        }

        // Select randomly from available locations
        let food_spot = *available_food_locations.choose(rng).unwrap();
        foods.push(food_spot);
    }

	// Finally, try to place 1 food in center of board for dramatic purposes
	foods.push(center);

	foods
}

pub fn static_board() -> Board {
    let mut snakes = ArrayVec::new();
    snakes.push(
        Snake {
            health: 100,
            body: vec![GridPoint {x: 1, y: 1}, GridPoint {x: 1, y: 1}, GridPoint {x: 1, y: 1}].into(),
        }
    );
    snakes.push(
        Snake {
            health: 100,
            body: vec![GridPoint {x: 9, y: 1}, GridPoint {x: 9, y: 1}, GridPoint {x: 9, y: 1}].into(),
        }
    );
    snakes.push(
        Snake {
            health: 100,
            body: vec![GridPoint {x: 9, y: 9}, GridPoint {x: 9, y: 9}, GridPoint {x: 9, y: 9}].into(),
        }
    );
    snakes.push(
        Snake {
            health: 100,
            body: vec![GridPoint {x: 1, y: 9}, GridPoint {x: 1, y: 9}, GridPoint {x: 1, y: 9}].into(),
        }
    );
    let foods = vec![CENTER];
    let board = Board::new(
        0,
        Some(foods),
        None,
        snakes,
    );
    board
}