use std::{rc::Rc, cell::RefCell, mem};

use crate::game::{
    Board,
    Snake,
    GridPoint,
    MAX_SNAKE_COUNT,
};

use super::{
    collector::{
        IndexType,
        SparseCollector,
        FeaturesHandler,
        ExamplesHandler,
        SIZE,
        ValueType
    },
    examples::ExamplesCollector, Example,
};


pub struct GlobalMetricsFeatures {
    collector: SparseCollector,
    offset: GlobalMetricsOffset,
}

impl GlobalMetricsFeatures {
    pub fn new(offset: GlobalMetricsOffset) -> GlobalMetricsFeatures {
        GlobalMetricsFeatures {
            collector: SparseCollector::new(),
            offset,
        }
    }
}

impl FeaturesHandler for GlobalMetricsFeatures {
    fn pre(&mut self, _board: &Board) {}
    
    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, _snake: &Snake, _alive_index: usize, _snake_index: usize, _board: &Board) {}
    
    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, board: &Board) {
        let size = SIZE as ValueType;
        
        // Food count [0.0; 1.0]
        let food_count = board.foods.len() as ValueType / size;
        self.collector.add(self.offset.get_food_count_index(), food_count);

        // Safe zone size [0.0; 1.0]
        let sz = &board.safe_zone;
        let safe_zone_size = ((sz.p0.x - sz.p1.x) * (sz.p0.y - sz.p1.y)) as ValueType / size;
        self.collector.add(self.offset.get_safe_zone_size_index(), safe_zone_size);

        let free_cells_count = board.objects.empties.len() as ValueType / size;
        self.collector.add(self.offset.get_free_cells_count_index(), free_cells_count);

        // TODO: Pass alive snakes in root, do not recalculate
        let alive_snakes_count = board.snakes.iter().filter(|snake| snake.is_alive()).count() as ValueType / MAX_SNAKE_COUNT as ValueType;
        self.collector.add(self.offset.get_alive_snakes_count_index(), alive_snakes_count);
    }

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
const FOOD_COUNT_AT: IndexType = 0;
const SAFE_ZONE_SIZE_AT: IndexType = FOOD_COUNT_AT + 1;
const FREE_CELLS_COUNT_AT: IndexType = SAFE_ZONE_SIZE_AT + 1;
const ALIVE_SNAKES_COUNT_AT: IndexType = FREE_CELLS_COUNT_AT + 1;
pub const NUM_FEATURES: IndexType = ALIVE_SNAKES_COUNT_AT + 1;


// Offset index calculations
fn calculate_food_count_at(offset: IndexType) -> IndexType {
    FOOD_COUNT_AT + offset
}

fn calculate_safe_zone_size_at(offset: IndexType) -> IndexType {
    SAFE_ZONE_SIZE_AT + offset
}

fn calculate_free_cells_count_at(offset: IndexType) -> IndexType {
    FREE_CELLS_COUNT_AT + offset
}

fn calculate_alive_snakes_count_at(offset: IndexType) -> IndexType {
    ALIVE_SNAKES_COUNT_AT + offset
}

#[derive(Debug)]
pub struct GlobalMetricsOffset {
    food_count_at: IndexType,
    safe_zone_size_at: IndexType,
    free_cells_count_at: IndexType,
    alive_snakes_count_at: IndexType,
}

impl GlobalMetricsOffset {
    pub fn new(offset: IndexType) -> GlobalMetricsOffset {
        GlobalMetricsOffset {
            food_count_at: calculate_food_count_at(offset),
            safe_zone_size_at: calculate_safe_zone_size_at(offset),
            free_cells_count_at: calculate_free_cells_count_at(offset),
            alive_snakes_count_at: calculate_alive_snakes_count_at(offset),
        }
    }

    fn get_food_count_index(&self) -> IndexType {
        self.food_count_at
    }

    fn get_safe_zone_size_index(&self) -> IndexType {
        self.safe_zone_size_at
    }

    fn get_free_cells_count_index(&self) -> IndexType {
        self.free_cells_count_at
    }

    fn get_alive_snakes_count_index(&self) -> IndexType {
        self.alive_snakes_count_at
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


pub struct GlobalMetricsExamples {
    examples_collector: Rc<RefCell<ExamplesCollector>>,
    offset: GlobalMetricsOffset,
}

impl GlobalMetricsExamples {
    pub fn new(examples_collector: Rc<RefCell<ExamplesCollector>>, offset: GlobalMetricsOffset) -> GlobalMetricsExamples {
        GlobalMetricsExamples {
            examples_collector,
            offset,
        }
    }
}

impl ExamplesHandler for GlobalMetricsExamples {
    fn pre(&mut self, _board: &Board) {}

    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, _snake: &Snake, _alive_index: usize, _snake_index: usize, _board: &Board) {}

    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, board: &Board) {
        let size = SIZE as ValueType;
        
        // Food count [0.0; 1.0]
        let food_count = board.foods.len() as ValueType / size;
        self.examples_collector.borrow_mut().parameter(
            self.offset.get_food_count_index(),
            food_count,
        );

        // Safe zone size [0.0; 1.0]
        let sz = &board.safe_zone;
        let safe_zone_size = ((sz.p0.x - sz.p1.x) * (sz.p0.y - sz.p1.y)) as ValueType / size;
        self.examples_collector.borrow_mut().parameter(
            self.offset.get_safe_zone_size_index(),
            safe_zone_size,
        );

        let free_cells_count = board.objects.empties.len() as ValueType / size;
        self.examples_collector.borrow_mut().parameter(
            self.offset.get_free_cells_count_index(),
            free_cells_count,
        );

        // TODO: Pass alive snakes in root, do not recalculate
        let alive_snakes_count = board.snakes.iter().filter(|snake| snake.is_alive()).count() as ValueType / MAX_SNAKE_COUNT as ValueType;
        self.examples_collector.borrow_mut().parameter(
            self.offset.get_alive_snakes_count_index(),
            alive_snakes_count,
        );
    }

    fn num_examples(&self) -> usize {
        self.examples_collector.borrow().len()
    }

    fn pop_examples(&mut self) -> Vec<Example> {
        self.examples_collector.borrow_mut().pop_examples()
    }
}
