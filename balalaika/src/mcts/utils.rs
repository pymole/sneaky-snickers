use crate::api::objects::Movement;
use crate::engine::MOVEMENTS;
use crate::game::{Board, Object, Point, MAX_SNAKE_COUNT, HEIGHT, WIDTH};
use arrayvec::ArrayVec;

use super::config::Config;
use super::search::Search;


pub fn get_masks(board: &Board) -> [[bool; 4]; MAX_SNAKE_COUNT] {
    // Returns movement masks for alive snakes

    let mut tails = ArrayVec::<_, MAX_SNAKE_COUNT>::new();
    
    for snake in board.snakes.iter() {
        let tail = snake.body[snake.body.len() - 1];
        let pretail = snake.body[snake.body.len() - 2];
        if tail != pretail {
            tails.push(tail);
        }
    }
    
    // TODO: use maybeuninit
    let mut masks = [[false; 4]; MAX_SNAKE_COUNT];

    board.snakes.iter()
        .enumerate()
        .for_each(|(snake_index, snake)| {
            MOVEMENTS
                .iter()
                .for_each(|&movement| {
                    let movement_position = get_movement_position(snake.head(), movement);
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

pub fn search<C: Config, S: Search<C>>(searcher: &mut S, board: &Board) {
    let config = searcher.get_config();
    if let Some(search_time) = config.get_search_time() {
        searcher.search_with_time(board, search_time);
    } else if let Some(iterations) = config.get_search_iterations() {
        searcher.search(board, iterations);
    } else {
        panic!("Provide MCTS_SEARCH_TIME or MCTS_ITERATIONS");
    }
}

pub fn get_best_movement<C: Config, S: Search<C>>(searcher: &mut S, board: &Board, agent: usize) -> Movement {
    search(searcher, board);
    searcher.get_final_movement(board, agent)
}


#[cfg(test)]
mod tests {
    use crate::{game::{Board, Snake, Point, Object}, mcts::utils::get_masks};

    #[test]
    fn test_get_masks() {
        let board = Board::new(
            0,
            Vec::new(),
            None,
            Some(&Vec::new()),
            [
                Snake {
                    health: 100,
                    body: [Point {x: 5, y: 4}, Point {x: 4, y: 4}, Point {x: 4, y: 4}].into()
                },
                Snake {
                    health: 100,
                    body: [Point {x: 9, y: 9}, Point {x: 9, y: 9}, Point {x: 9, y: 9}].into()
                },
            ],
        );

        let masks = get_masks(&board);
        assert_eq!(board.objects[(4, 4)], Object::BodyPart);
        assert_eq!(masks[0], [true, true, true, false]);
    }
}
