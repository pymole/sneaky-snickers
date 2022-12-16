use crate::game::{GridPoint, Snake, Board, self, WIDTH, HEIGHT, Point};

// i64 for torch sparse tensor
pub type IndexType = i64;
pub type ValueType = f32;

pub const SIZE: IndexType = game::SIZE as IndexType;
pub const MAX_SNAKE_COUNT: IndexType = game::MAX_SNAKE_COUNT as IndexType;
pub const ALL_PLAYERS_GRIDS_SIZE: IndexType = MAX_SNAKE_COUNT * SIZE;

#[derive(Clone)]
pub struct SparseCollector {
    pub indices: Vec<IndexType>,
    pub values: Vec<ValueType>,
}

impl SparseCollector {
    pub fn new() -> SparseCollector {
        SparseCollector {
            indices: Vec::new(),
            values: Vec::new(),
        }
    }
    pub fn add(&mut self, i: IndexType, v: ValueType) {
        self.indices.push(i);
        self.values.push(v);
    }
}

// TODO: I've had problems with extracting common methods in another trait
pub trait FeaturesHandler {
    fn pre(&mut self, board: &Board);
    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, snake_index: usize, board: &Board);
    fn snake(&mut self, snake: &Snake, snake_index: usize, board: &Board);
    fn hazard(&mut self, pos: GridPoint);
    fn food(&mut self, pos: GridPoint);
    fn post(&mut self, board: &Board);
    fn num_features(&self) -> IndexType;
    fn pop_features(&mut self) -> (Vec<IndexType>, Vec<ValueType>);
}

pub trait ExamplesHandler {
    fn pre(&mut self, board: &Board);
    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, snake_index: usize, board: &Board);
    fn snake(&mut self, snake: &Snake, snake_index: usize, board: &Board);
    fn hazard(&mut self, pos: GridPoint);
    fn food(&mut self, pos: GridPoint);
    fn post(&mut self, board: &Board);
    fn num_examples(&self) -> usize;
    fn pop_examples(&mut self) -> Vec<Example>;
}

pub type Example = ((Vec<IndexType>, Vec<ValueType>), Rewards);

pub fn collect_features(board: &Board, handler: &mut impl FeaturesHandler) {
    assert!(!board.is_terminal());
    handler.pre(board);

    let mut alive_index = 0;

    for (snake_index, snake) in board.snakes.iter().enumerate() {
        handler.snake(snake, snake_index, board);
        if snake.is_alive() {
            handler.alive_snake(snake, alive_index, snake_index, board);
        }
        alive_index += 1;
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        handler.food(food);
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                handler.hazard(p);
            }
        }
    }

    handler.post(board);
}


pub fn collect_examples(board: &Board, handler: &mut impl ExamplesHandler) {
    assert!(!board.is_terminal());
    handler.pre(board);
    
    let mut alive_index = 0;

    for (snake_index, snake) in board.snakes.iter().enumerate() {
        handler.snake(snake, snake_index, board);
        if snake.is_alive() {
            handler.alive_snake(snake, alive_index, snake_index, board);
            alive_index += 1;
        }
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        handler.food(food);
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                handler.hazard(p);
            }
        }
    }

    handler.post(board);
}

pub type Rewards = [f32; game::MAX_SNAKE_COUNT];

pub fn get_rewards(board: &Board) -> (Rewards, bool) {
    debug_assert!(board.is_terminal());
    let mut rewards = [0.0; game::MAX_SNAKE_COUNT];
    let mut draw = true;
    for (i, snake) in board.snakes.iter().enumerate() {
        if snake.is_alive() {
            draw = false;
            rewards[i] = 1.0;
            break;
        }
    }
    (rewards, draw)
}

pub fn get_point_index(p: GridPoint) -> IndexType {
    (p.y * HEIGHT + p.x) as IndexType
}