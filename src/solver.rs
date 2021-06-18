use std::collections::HashSet;
use crate::objects::{State, Movement, Point};

const MOVEMENTS: [Movement; 4] = [
    Movement::Left,
    Movement::Right,
    Movement::Up,
    Movement::Down
];


pub fn get_best_movement(state: State) -> Movement {
    let head = state.you.head;
    let nearest_food = get_nearest_target(head, state.board.food);

    let mut busy_positions = HashSet::new();
    for snake in state.board.snakes {
        for part in snake.body {
            busy_positions.insert(part);
        }
    }

    let mut best_movement = Movement::Right;
    let mut food_distance = f64::MAX;
    for &movement in &MOVEMENTS {
        let movement_position = get_movement_position(head, movement);
        if !busy_positions.contains(&movement_position)
            && !is_outside_of_board(state.board.width, state.board.height, movement_position) {
            let distance = distance(movement_position, nearest_food);

            if distance < food_distance {
                food_distance = distance;
                best_movement = movement;
            }
        }
    }

    best_movement
}

fn get_nearest_target(position: Point, targets: Vec<Point>) -> Point {
    let mut nearest = targets[0];
    let mut nearest_distance = f64::MAX;

    for target in targets {
        let distance = distance(position, target);
        if distance < nearest_distance {
            nearest = target;
            nearest_distance = distance;
        }
    }

    nearest
}

fn get_movement_position(position: Point, movement: Movement) -> Point {
    match movement {
        Movement::Right => Point {x: position.x + 1, y: position.y},
        Movement::Left => Point {x: position.x - 1, y: position.y},
        Movement::Up => Point {x: position.x, y: position.y + 1},
        Movement::Down => Point {x: position.x, y: position.y - 1},
    }
}

fn is_outside_of_board(width: i32, height: i32, position: Point) -> bool{
    position.x < 0 || position.x >= width || position.y < 0 || position.y >= height
}

fn distance(p1: Point, p2: Point) -> f64 {
    (((p1.x - p2.x).pow(2) + (p1.y - p2.y).pow(2)) as f64).sqrt()
}
