use std::{rc::Rc, cell::RefCell, mem};

use crate::{game::{
    Board,
    Snake,
    GridPoint,
    self,
}, mcts::heuristics::flood_fill::flood_fill};

use super::{
    collector::{
        IndexType,
        SparseCollector,
        FeaturesHandler,
        ExamplesHandler,
        ValueType, self, Rewards
    },
    examples::ExamplesCollector, Example,
};


pub struct FloodFillFeatures {
    collector: SparseCollector,
    offset: FloodFillOffset,
    flood_fill: Rewards,
}

impl FloodFillFeatures {
    pub fn new(offset: FloodFillOffset) -> FloodFillFeatures {
        FloodFillFeatures {
            collector: SparseCollector::new(),
            offset,
            flood_fill: [0.0; game::MAX_SNAKE_COUNT],
        }
    }
}

impl FeaturesHandler for FloodFillFeatures {
    fn pre(&mut self, board: &Board) {
        self.flood_fill = flood_fill(board);
    }

    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, _snake: &Snake, _alive_index: usize, snake_index: usize, _board: &Board) {
        self.collector.add(self.offset.get_flood_fill_index(snake_index), self.flood_fill[snake_index]);
    }
    
    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, _board: &Board) {}

    fn num_features(&self) -> IndexType {
        NUM_FEATURES
    }

    fn pop_features(&mut self) -> (Vec<IndexType>, Vec<ValueType>) {
        let indices = mem::replace(&mut self.collector.indices, Vec::new());
        let values = mem::replace(&mut self.collector.values, Vec::new());
        (indices, values)
    }
}

// Indexes
const FLOOD_FILL_AT: IndexType = 0;
pub const NUM_FEATURES: IndexType = FLOOD_FILL_AT + collector::MAX_SNAKE_COUNT;

// Offset index calculations
fn calculate_flood_fill_at(offset: IndexType) -> IndexType {
    offset + FLOOD_FILL_AT
}

#[derive(Debug)]
pub struct FloodFillOffset {
    flood_fill_at: IndexType,
}

impl FloodFillOffset {
    pub fn new(offset: IndexType) -> FloodFillOffset {
        FloodFillOffset {
            flood_fill_at: calculate_flood_fill_at(offset),
        }
    }

    fn get_flood_fill_index(&self, owner: usize) -> IndexType {
        self.flood_fill_at + owner as IndexType
    }
}


/// Rotates, flips and permutates snakes
/// Index pattern:
/// | rot                       | rot                       |
/// | flip        | flip        | flip        | flip        |
/// | perm | perm | perm | perm | perm | perm | perm | perm |
/// Rotations: 4
/// Flips on rotation: 3
/// Snake positions permutations: A(n=MAX_SNAKES_COUNT, k=alive_snakes_count)


pub struct FloodFillExamples {
    examples_collector: Rc<RefCell<ExamplesCollector>>,
    offset: FloodFillOffset,
    flood_fill: Rewards,
}

impl FloodFillExamples {
    pub fn new(examples_collector: Rc<RefCell<ExamplesCollector>>, offset: FloodFillOffset) -> FloodFillExamples {
        FloodFillExamples {
            examples_collector,
            offset,
            flood_fill: [0.0; game::MAX_SNAKE_COUNT],
        }
    }
}

impl ExamplesHandler for FloodFillExamples {
    fn pre(&mut self, board: &Board) {
        self.flood_fill = flood_fill(board);
    }
    
    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, _snake: &Snake, alive_index: usize, snake_index: usize, _board: &Board) {
        let mut examples_collector = self.examples_collector.borrow_mut();
        let value = self.flood_fill[snake_index];
        examples_collector.alive_snake_parameter_filler(
            alive_index,
            |collector, owner| {
                // TODO: Check if it is normal to skip
                collector.add(self.offset.get_flood_fill_index(owner), value);
            }
        );
    }
    
    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, _board: &Board) {}

    fn num_examples(&self) -> usize {
        self.examples_collector.borrow().len()
    }

    fn pop_examples(&mut self) -> Vec<Example> {
        self.examples_collector.borrow_mut().pop_examples()
    }
}
