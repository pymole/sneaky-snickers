use std::collections::{HashMap, HashSet};

use crate::api::objects::{Movement, State};
use crate::game::{Board, Point};
use crate::engine::MOVEMENTS;

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

fn is_outside_of_board(width: i32, height: i32, position: Point) -> bool {
    position.x < 0 || position.x >= width || position.y < 0 || position.y >= height
}

fn distance(p1: Point, p2: Point) -> f64 {
    (((p1.x - p2.x).pow(2) + (p1.y - p2.y).pow(2)) as f64).sqrt()
}


pub type FloodFill = HashMap<usize, Vec<Point>>;
pub fn flavored_flood_fill(board: &Board) -> FloodFill {
    let mut body_part_empty_at = HashMap::new();
    let mut sizes = Vec::with_capacity(board.snakes.len());

    let mut flood_fronts = Vec::with_capacity(board.snakes.len());
    let mut seized_points = vec![vec![]; board.snakes.len()];
    
    for snake in board.snakes.iter() {
        if snake.health <= 0 {
            continue;
        }

        for (empty_at, body_part) in snake.body.iter().rev().enumerate() {
            body_part_empty_at.insert(body_part, empty_at + 1);
        }

        sizes.push(snake.body.len());
        flood_fronts.push(vec![snake.body[0]]);
    }

    let mut visited = HashSet::new();
    let mut turn = 1;

    loop {
        let mut contenders_at_point = HashMap::<Point, Vec<usize>>::new();
        
        for (snake_index, flood_front) in flood_fronts.iter_mut().enumerate() {
            while let Some(point) = flood_front.pop() {
                for movement_position in movement_positions(board.size.x, board.size.y, point) {
                    if visited.contains(&movement_position) {
                        // Somebody already seized this point
                        continue;
                    }
                    if let Some(contenders) = contenders_at_point.get(&movement_position) {
                        // Already contesting for this point
                        if contenders.contains(&snake_index) {
                            continue;
                        }
                    }
                    if let Some(empty_at) = body_part_empty_at.get(&movement_position) {
                        // This body part is not ready for contest because it's not empty yet
                        if turn < *empty_at {
                            continue;
                        }
                    }

                    contenders_at_point
                        .entry(movement_position)
                        .or_insert(Vec::new())
                        .push(snake_index);
                }
            }
        }
        
        let mut all_fronts_empty = true;
        for (point, contenders) in contenders_at_point {
            visited.insert(point);

            let mut winner_id;
            if contenders.len() == 1 {
                winner_id = contenders[0];
            } else {
                // Largest snake get the point
                winner_id = 0;
                let mut largest_size = 0;
                let mut several_largest = false;
                
                for contender in contenders {
                    let size = sizes[contender];
                    if largest_size < size {
                        winner_id = contender;
                        several_largest = false;
                        largest_size = size;
                    } else if largest_size == size {
                        several_largest = true;
                    }
                }

                // If there is multiple snakes that bigger than others then no one get the point
                if several_largest {
                    continue;
                }
            }

            let flood_front = &mut flood_fronts[winner_id];
            let seized = &mut seized_points[winner_id];
            flood_front.push(point);
            seized.push(point);
            all_fronts_empty = false;
        }

        if all_fronts_empty {
            break;
        }

        turn += 1;
    }

    (0..board.snakes.len())
        .filter(|&snake_id| board.snakes[snake_id].is_alive())
        .zip(seized_points)
        .collect()
}

pub fn flavored_flood_fill_estimate(board: &Board) -> Vec<(usize, f32)> {
    let fff = flavored_flood_fill(board);
    let squares_count = board.objects.len1 * board.objects.len2;

    fff.into_iter()
        .map(|(id, seized)| (id, (seized.len()/squares_count) as f32))
        .collect()
}

fn movement_positions(width: i32, height: i32, position: Point) -> Vec<Point> {
    let mut positions = Vec::new();
    if position.x < width - 1 {
        positions.push(Point {x: position.x + 1, y: position.y});
    }
    if position.x > 0 {
        positions.push(Point {x: position.x - 1, y: position.y});
    }
    if position.y < height - 1 {
        positions.push(Point {x: position.x, y: position.y + 1});
    }
    if position.y > 0 {
        positions.push(Point {x: position.x, y: position.y - 1});
    }

    positions
}