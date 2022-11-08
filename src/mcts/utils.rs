use crate::api::objects::Movement;
use crate::engine::MOVEMENTS;
use crate::game::{Board, Object, Point, MAX_SNAKE_COUNT, HEIGHT, WIDTH};
use arrayvec::ArrayVec;


pub fn get_masks(board: &Board) -> [[bool; 4]; MAX_SNAKE_COUNT] {
    let tails: ArrayVec<_, MAX_SNAKE_COUNT> = board.snakes
        .iter()
        .filter(|snake| snake.is_alive())
        .map(|snake| *snake.body.back().unwrap())
        .collect();
    
    let mut masks = [[false; 4]; MAX_SNAKE_COUNT];

    board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .for_each(|(snake_index, snake)| {
            MOVEMENTS
                .iter()
                .for_each(|&movement| {
                    let movement_position = get_movement_position(snake.body[0], movement);
                    if board.objects[movement_position] != Object::BodyPart || tails.contains(&movement_position) {
                        masks[snake_index][movement as usize] = true;
                    }
                });            
        });
    
    masks
}

pub fn get_movement_position(position: Point, movement: Movement) -> Point {
    match movement {
        Movement::Right => Point {x: (position.x + 1) % WIDTH, y: position.y},
        Movement::Left => Point {x: (WIDTH + position.x - 1) % WIDTH, y: position.y},
        Movement::Up => Point {x: position.x, y: (position.y + 1) % HEIGHT },
        Movement::Down => Point {x: position.x, y: (HEIGHT + position.y - 1) % HEIGHT},
    }
}

pub fn movement_positions(position: Point) -> [Point; 4] {
    let positions = [
        Point {x: position.x, y: (position.y + 1) % HEIGHT },
        Point {x: (position.x + 1) % WIDTH, y: position.y},
        Point {x: position.x, y: (HEIGHT + position.y - 1) % HEIGHT},
        Point {x: (WIDTH + position.x - 1) % WIDTH, y: position.y},
    ];

    positions
}