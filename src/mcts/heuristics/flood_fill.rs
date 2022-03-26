use crate::{
    game::{
        Point, Board, MAX_SNAKE_COUNT, WIDTH, HEIGHT, SIZE
    },
    mcts::utils::movement_positions
};

pub type FloodFill = [Vec<Point>; MAX_SNAKE_COUNT];

#[derive(Clone)]
struct ContendersInfo {
    several_largest: bool,
    winner_index: usize,
    largest_size: usize,
    visited: bool,
    body_part_empty_at: usize,
}

impl Default for ContendersInfo {
    fn default() -> Self {
        ContendersInfo {
            several_largest: false,
            winner_index: usize::MAX,
            largest_size: 0,
            visited: false,
            body_part_empty_at: 0,
        }
    }
}

pub fn flood_fill(board: &Board) -> FloodFill {
    let mut sizes = [0; MAX_SNAKE_COUNT];

    let mut flood_fronts: [Vec<Point>; MAX_SNAKE_COUNT] = Default::default();
    let mut seized_points: [Vec<Point>; MAX_SNAKE_COUNT] = Default::default();

    let mut contenders_at_point: [[ContendersInfo; HEIGHT]; WIDTH] = Default::default();

    for (i, snake) in board.snakes.iter().enumerate() {
        if !snake.is_alive() {
            continue;
        }

        for (empty_at, body_part) in snake.body.iter().rev().enumerate() {
            contenders_at_point[body_part.x as usize][body_part.y as usize].body_part_empty_at = empty_at + 1;
        }
        
        seized_points[i].reserve_exact(SIZE);
        flood_fronts[i].reserve_exact(SIZE);

        sizes[i] = snake.body.len();
        flood_fronts[i].push(snake.body[0]);
    }

    let mut turn = 1;
    let mut current_points = Vec::new();

    loop {
        for (snake_index, flood_front) in flood_fronts.iter_mut().enumerate() {
            while let Some(point) = flood_front.pop() {
                for movement_position in movement_positions(point, board.size) {
                    let contenders_info = &mut contenders_at_point[movement_position.x as usize][movement_position.y as usize];

                    if contenders_info.visited {
                        // This point already contended
                        continue;
                    }

                    if turn < contenders_info.body_part_empty_at {
                        // This body part is not ready for contest because it's not empty yet
                        continue;
                    }

                    if !current_points.contains(&movement_position) {
                        current_points.push(movement_position);
                    }
                    
                    let size = sizes[snake_index];
                    if contenders_info.largest_size < size {
                        contenders_info.largest_size = size;
                        contenders_info.winner_index = snake_index;
                        contenders_info.several_largest = false;
                    } else if contenders_info.largest_size == size && contenders_info.winner_index != snake_index {
                        contenders_info.several_largest = true;
                    }
                }
            }
        }

        let mut all_fronts_empty = true;

        for &point in &current_points {
            let contenders_info = &mut contenders_at_point[point.x as usize][point.y as usize];
            contenders_info.visited = true;

            if !contenders_info.several_largest {
                let flood_front = &mut flood_fronts[contenders_info.winner_index];
                let seized = &mut seized_points[contenders_info.winner_index];
                flood_front.push(point);
                seized.push(point);
                all_fronts_empty = false;
            }
        }

        if all_fronts_empty {
            break;
        }

        current_points.clear();

        turn += 1;
    }

    // info!("{:?}", Instant::now() - start);

    // for y in (0..HEIGHT).rev() {
    //     let v: Vec<_> = (0..WIDTH).map(|x| {
    //         let c = &contenders_at_point[x][y];
    //         if c.visited {
    //             c.winner_index
    //         } else {
    //             123123
    //         }
    //     }).collect();
    //     info!("{:?}", v);
    // }

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
