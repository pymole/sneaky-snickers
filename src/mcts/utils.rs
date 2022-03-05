use crate::api::objects::Movement;
use crate::engine::MOVEMENTS;
use crate::game::{Board, Object, Point};

pub fn get_masks(board: &Board) -> Vec<(usize, [bool; 4])> {
    let tails: Vec<_> = board.snakes
        .iter()
        .filter(|snake| snake.is_alive())
        .map(|snake| *snake.body.back().unwrap())
        .collect();

    board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .map(|(snake_id, snake)| {
            let mut movement_mask = [false; 4];
            MOVEMENTS
                .iter()
                .for_each(|&movement| {
                    let movement_position = get_movement_position(snake.body[0], movement, board.size);
                    if tails.contains(&movement_position) || board.objects[movement_position] != Object::BodyPart {
                        movement_mask[movement as usize] = true;
                    }
                });
            (snake_id, movement_mask)
        })
        .collect()
}

pub fn get_movement_position(position: Point, movement: Movement, borders: Point) -> Point {
    match movement {
        Movement::Right => Point {x: (position.x + 1) % borders.x, y: position.y},
        Movement::Left => Point {x: (borders.x + position.x - 1) % borders.x, y: position.y},
        Movement::Up => Point {x: position.x, y: (position.y + 1) % borders.y },
        Movement::Down => Point {x: position.x, y: (borders.y + position.y - 1) % borders.y},
    }
}