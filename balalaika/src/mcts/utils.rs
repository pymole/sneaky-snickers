use std::env;
use std::str::FromStr;
use std::time::Duration;

use arrayvec::ArrayVec;
use rand::{seq::SliceRandom, Rng};

use crate::{api::objects::Movement, game::GridPoint};
use crate::engine::MOVEMENTS;
use super::search::Search;
use crate::game::{Board, Point, MAX_SNAKE_COUNT, HEIGHT, WIDTH};


pub fn parse_env<Value>(key: &str) -> Option<Value>
where
    Value: FromStr,
    <Value as FromStr>::Err: std::fmt::Debug
{
    env::var(key).map(|s| s.parse().expect("MCTSConfig: can't parse env variable")).ok()
}

#[derive(Clone, Copy)]
pub struct SearchOptions {
    pub iterations: Option<usize>,
    pub search_time: Option<Duration>,
    pub verbose: bool,
}

impl SearchOptions {
    pub fn from_env() -> SearchOptions {
        let config = SearchOptions {
            iterations: parse_env("MCTS_ITERATIONS"),
            search_time: parse_env("MCTS_SEARCH_TIME").map(Duration::from_millis),
            verbose: env::var("MCTS_VERBOSE").is_ok(),
        };

        config
    }
}

pub fn get_masks(board: &Board) -> [[bool; 4]; MAX_SNAKE_COUNT] {
    // Returns movement masks for alive snakes

    let mut tails = ArrayVec::<_, MAX_SNAKE_COUNT>::new();
    
    for snake in board.snakes.iter() {
        if !snake.is_alive() {
            continue;
        }
        let tail = snake.body[snake.body.len() - 1];
        let pretail = snake.body[snake.body.len() - 2];
        if tail != pretail {
            tails.push(tail);
        }
    }
    
    let mut masks = [[false; 4]; MAX_SNAKE_COUNT];

    board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .for_each(|(snake_index, snake)| {
            MOVEMENTS
                .iter()
                .for_each(|&movement| {
                    let movement_position = get_movement_position(snake.head(), movement);
                    if !board.objects.is_body(movement_position.into()) || tails.contains(&movement_position) {
                        masks[snake_index][movement as usize] = true;
                    }
                });
        });
    
    masks
}

pub fn get_movement_position(position: GridPoint, movement: Movement) -> GridPoint {
    match movement {
        Movement::Right => Point {x: (position.x + 1) % WIDTH, y: position.y},
        Movement::Left => Point {x: (WIDTH + position.x - 1) % WIDTH, y: position.y},
        Movement::Up => Point {x: position.x, y: (position.y + 1) % HEIGHT },
        Movement::Down => Point {x: position.x, y: (HEIGHT + position.y - 1) % HEIGHT},
    }
}

pub fn movement_positions_wrapped(position: GridPoint) -> [GridPoint; 4] {
    let positions = [
        Point {x: position.x, y: (position.y + 1) % HEIGHT },
        Point {x: (position.x + 1) % WIDTH, y: position.y},
        Point {x: position.x, y: (HEIGHT + position.y - 1) % HEIGHT},
        Point {x: (WIDTH + position.x - 1) % WIDTH, y: position.y},
    ];

    positions
}

pub fn movement_positions_standard(position: GridPoint) -> [GridPoint; 4] {
    let positions = [
        Point {x: position.x, y: position.y + 1 },
        Point {x: position.x + 1, y: position.y},
        Point {x: position.x, y: position.y - 1},
        Point {x: position.x - 1, y: position.y},
    ];

    positions
}

pub fn get_random_actions_from_masks(random: &mut impl Rng, board: &Board) -> [usize; MAX_SNAKE_COUNT] {
    let masks = get_masks(board);
    let mut actions = [0; MAX_SNAKE_COUNT];

    for i in 0..board.snakes.len() {
        if !board.snakes[i].is_alive() {
            continue;
        }
        let available_actions = masks[i]
            .into_iter()
            .enumerate()
            .filter(|(_, mask)| *mask)
            .map(|(movement, _)| movement)
            .collect::<Vec<_>>();
        
        if let Some(&action) = available_actions.choose(random) {
            actions[i] = action;
        }
    }

    actions
}

pub fn get_first_able_actions_from_masks(board: &Board) -> [usize; MAX_SNAKE_COUNT] {
    let masks = get_masks(board);
    let mut actions = [0; MAX_SNAKE_COUNT];

    for i in 0..board.snakes.len() {
        if !board.snakes[i].is_alive() {
            continue;
        }
        let action = masks[i]
            .into_iter()
            .enumerate()
            .filter(|(_, mask)| *mask)
            .map(|x| x.0)
            .next()
            .unwrap_or(0);
        
        actions[i] = action;
    }
    actions
}

pub fn search(searcher: &mut impl Search, board: &Board, options: SearchOptions) {
    if let Some(search_time) = options.search_time {
        searcher.search_with_time(board, search_time, options.verbose);
    } else if let Some(iterations) = options.iterations {
        searcher.search(board, iterations, options.verbose);
    } else {
        panic!("Provide MCTS_SEARCH_TIME or MCTS_ITERATIONS");
    }
}

pub fn get_best_movement(searcher: &mut impl Search, board: &Board, agent: usize, options: SearchOptions) -> Movement {
    search(searcher, board, options);
    searcher.get_final_movement(board, agent, options.verbose)
}


#[cfg(test)]
mod tests {
    use arrayvec::ArrayVec;

    use crate::{game::{Board, Snake, Point}, mcts::utils::get_masks};

    #[test]
    fn test_get_masks() {
        let mut snakes = ArrayVec::new();
        snakes.push(
            Snake {
                health: 100,
                body: [Point {x: 0, y: 6}, Point {x: 0, y: 6}, Point {x: 0, y: 6}].into()
            }
        );
        snakes.push(
            Snake {
                health: 100,
                body: [Point {x: 1, y: 6}, Point {x: 1, y: 6}, Point {x: 1, y: 6}].into()
            },
        );

        let board = Board::new(
            0,
            Some(vec![Point {x: 0, y: 0}]),
            None,
            snakes,
        );

        let masks = get_masks(&board);
        assert_eq!(masks[0], [true, false, true, true]);
        assert_eq!(masks[1], [true, true, true, false]);
    }
}
