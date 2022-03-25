use arrayvec::ArrayVec;
use std::collections::HashMap;

use crate::{game::{Point, Board, MAX_SNAKE_COUNT, WIDTH, HEIGHT}, mcts::utils::movement_positions};

pub type FloodFill = [Vec<Point>; MAX_SNAKE_COUNT];

pub fn flood_fill(board: &Board) -> FloodFill {
    let mut body_part_empty_at = HashMap::new();
    let mut sizes = [0; MAX_SNAKE_COUNT];

    let mut flood_fronts: [Vec<Point>; MAX_SNAKE_COUNT] = Default::default();
    let mut seized_points: [Vec<Point>; MAX_SNAKE_COUNT] = Default::default();

    for (i, snake) in board.snakes.iter().enumerate() {
        if !snake.is_alive() {
            continue;
        }

        for (empty_at, body_part) in snake.body.iter().rev().enumerate() {
            body_part_empty_at.insert(body_part, empty_at + 1);
        }

        sizes[i] = snake.body.len();
        flood_fronts[i] = vec![snake.body[0]];
    }

    let mut visited = [[false; HEIGHT]; WIDTH];
    let mut turn = 1;

    loop {
        let mut contenders_at_point = HashMap::<Point, ArrayVec<usize, MAX_SNAKE_COUNT>>::new();

        for (snake_index, flood_front) in flood_fronts.iter_mut().enumerate() {
            while let Some(point) = flood_front.pop() {
                for movement_position in movement_positions(point, board.size) {
                    if visited[movement_position.x as usize][movement_position.y as usize] {
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
                        .or_insert(ArrayVec::new())
                        .push(snake_index);
                }
            }
        }

        let mut all_fronts_empty = true;
        for (point, contenders) in contenders_at_point {
            visited[point.x as usize][point.y as usize] = true;

            let mut winner_index;
            if contenders.len() == 1 {
                winner_index = contenders[0];
            } else {
                // Largest snake get the point
                winner_index = 0;
                let mut largest_size = 0;
                let mut several_largest = false;

                for contender in contenders {
                    let size = sizes[contender];
                    if largest_size < size {
                        winner_index = contender;
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

            let flood_front = &mut flood_fronts[winner_index];
            let seized = &mut seized_points[winner_index];
            flood_front.push(point);
            seized.push(point);
            all_fronts_empty = false;
        }

        if all_fronts_empty {
            break;
        }

        turn += 1;
    }

    seized_points
}

#[allow(dead_code)]
pub fn flood_fill_estimate(board: &Board) -> [f32; MAX_SNAKE_COUNT] {
    let flood_fill = flood_fill(board);
    let squares_count = (board.objects.len1 * board.objects.len2) as f32;

    let mut estimates = [0.0; MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        estimates[i] = flood_fill[i].len() as f32 / squares_count;
    }

    estimates
}
