use std::{rc::Rc, cell::RefCell, mem};

use crate::game::{
    Board,
    Snake,
    GridPoint,
    self,
};

use super::{
    collector::{
        IndexType,
        SparseCollector,
        FeaturesHandler,
        ExamplesHandler,
        ValueType, self
    },
    examples::ExamplesCollector, Example,
};


pub struct SnakeMetricsFeatures {
    collector: SparseCollector,
    offset: SnakeMetricsOffset,
    lengthes: [i32; game::MAX_SNAKE_COUNT],
    healthes: [i32; game::MAX_SNAKE_COUNT],
}

impl SnakeMetricsFeatures {
    pub fn new(offset: SnakeMetricsOffset) -> SnakeMetricsFeatures {
        SnakeMetricsFeatures {
            collector: SparseCollector::new(),
            offset,
            lengthes: [0; game::MAX_SNAKE_COUNT],
            healthes: [0; game::MAX_SNAKE_COUNT],
        }
    }
}

impl FeaturesHandler for SnakeMetricsFeatures {
    fn pre(&mut self, _board: &Board) {}

    fn snake(&mut self, snake: &Snake, snake_index: usize, _board: &Board) {
        self.lengthes[snake_index] = snake.is_alive() as i32 * snake.body.len() as i32;
        self.healthes[snake_index] = snake.health;
    }

    fn alive_snake(&mut self, snake: &Snake, _alive_index: usize, snake_index: usize, _board: &Board) {
        let health = snake.health as ValueType / 100.0;
        self.collector.add(self.offset.get_linear_health_index(snake_index), health);

        let length = snake.body.len() as ValueType / SIZE;
        self.collector.add(self.offset.get_length_index(snake_index), length);
    }
    
    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, _board: &Board) {
        for i in 0..game::MAX_SNAKE_COUNT - 1 {
            for j in i + 1..game::MAX_SNAKE_COUNT {
                let diff_len = (self.lengthes[i] - self.lengthes[j]) as f32 / SIZE;
                self.collector.add(self.offset.get_length_difference_index(i, j - 1), diff_len);
                self.collector.add(self.offset.get_length_difference_index(j, i), -diff_len);
                
                let diff_health = (self.healthes[i] - self.healthes[j]) as f32 / 100.0;
                self.collector.add(self.offset.get_health_difference_index(i, j - 1), diff_health);
                self.collector.add(self.offset.get_health_difference_index(j, i), -diff_health);
            }
        }
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


// Helper data
const SIZE: ValueType = collector::SIZE as ValueType;
const OTHER_SNAKES_COUNT: IndexType = collector::MAX_SNAKE_COUNT - 1;

// Indexes
const LINEAR_HEALTH_AT: IndexType = 0;
const LENGTHES_AT: IndexType = LINEAR_HEALTH_AT + collector::MAX_SNAKE_COUNT;
const LENGTH_DIFFERENCES_AT: IndexType = LENGTHES_AT + collector::MAX_SNAKE_COUNT;
const HEALTH_DIFFERENCES_AT: IndexType = LENGTH_DIFFERENCES_AT + collector::MAX_SNAKE_COUNT * OTHER_SNAKES_COUNT;
pub const NUM_FEATURES: IndexType = HEALTH_DIFFERENCES_AT + collector::MAX_SNAKE_COUNT * OTHER_SNAKES_COUNT;

// Offset index calculations
fn calculate_linear_health_at(offset: IndexType) -> IndexType {
    offset + LINEAR_HEALTH_AT
}

fn calculate_lengthes_at(offset: IndexType) -> IndexType {
    offset + LENGTHES_AT
}

fn calculate_length_differences_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut length_differences_at = [0; game::MAX_SNAKE_COUNT];
    
    for i in 0..collector::MAX_SNAKE_COUNT {
        length_differences_at[i as usize] = offset + LENGTH_DIFFERENCES_AT + i * OTHER_SNAKES_COUNT;
    }
    length_differences_at
}

fn calculate_health_differences_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut health_differences_at = [0; game::MAX_SNAKE_COUNT];
    
    for i in 0..collector::MAX_SNAKE_COUNT {
        health_differences_at[i as usize] = offset + HEALTH_DIFFERENCES_AT + i * OTHER_SNAKES_COUNT;
    }
    health_differences_at
}

#[derive(Debug)]
pub struct SnakeMetricsOffset {
    linear_health_at: IndexType,
    lengthes_at: IndexType,
    length_differences_at: [IndexType; game::MAX_SNAKE_COUNT],
    health_differences_at: [IndexType; game::MAX_SNAKE_COUNT],
}

impl SnakeMetricsOffset {
    pub fn new(offset: IndexType) -> SnakeMetricsOffset {
        SnakeMetricsOffset {
            linear_health_at: calculate_linear_health_at(offset),
            lengthes_at: calculate_lengthes_at(offset),
            length_differences_at: calculate_length_differences_at(offset),
            health_differences_at: calculate_health_differences_at(offset),
        }
    }

    fn get_linear_health_index(&self, owner: usize) -> IndexType {
        self.linear_health_at + owner as IndexType
    }

    fn get_length_index(&self, owner: usize) -> IndexType {
        self.lengthes_at + owner as IndexType
    }

    fn get_length_difference_index(&self, owner: usize, relative_other: usize) -> IndexType {
        self.length_differences_at[owner] + relative_other as IndexType
    }

    fn get_health_difference_index(&self, owner: usize, relative_other: usize) -> IndexType {
        self.health_differences_at[owner] + relative_other as IndexType
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


pub struct SnakeMetricsExamples {
    examples_collector: Rc<RefCell<ExamplesCollector>>,
    offset: SnakeMetricsOffset,
}

impl SnakeMetricsExamples {
    pub fn new(examples_collector: Rc<RefCell<ExamplesCollector>>, offset: SnakeMetricsOffset) -> SnakeMetricsExamples {
        SnakeMetricsExamples {
            examples_collector,
            offset,
        }
    }
}

impl ExamplesHandler for SnakeMetricsExamples {
    fn pre(&mut self, _board: &Board) {}

    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, _snake_index: usize, _board: &Board) {
        let mut examples_collector = self.examples_collector.borrow_mut();

        let health = snake.health as ValueType / 100.0;
        examples_collector.alive_snake_parameter_filler(
            alive_index,
            |collector, owner| {
                collector.add(self.offset.get_linear_health_index(owner), health);
            },
        );

        let length = snake.body.len() as ValueType / SIZE;
        examples_collector.alive_snake_parameter_filler(
            alive_index,
            |collector, owner| {
                collector.add(self.offset.get_length_index(owner), length);
            },
        );
    }

    fn food(&mut self, _pos: GridPoint) {}

    fn hazard(&mut self, _pos: GridPoint) {}

    fn post(&mut self, board: &Board) {
        let mut examples_collector = self.examples_collector.borrow_mut();

        let alive_snakes = examples_collector.context.alive_snakes.clone();
        
        examples_collector.alive_snakes_permutations_filler(|collector, permutation| {
            let mut lengthes = [0; game::MAX_SNAKE_COUNT];
            let mut healthes = [0; game::MAX_SNAKE_COUNT];

            for (origin, new) in alive_snakes.iter().copied().zip(permutation.iter().copied()) {
                lengthes[new] = board.snakes[origin].body.len() as i32;
                healthes[new] = board.snakes[origin].health;
            }

            for i in 0..game::MAX_SNAKE_COUNT - 1 {
                for j in i + 1..game::MAX_SNAKE_COUNT {
                    let diff_length = (lengthes[i] - lengthes[j]) as f32 / SIZE;
                    collector.add(self.offset.get_length_difference_index(i, j - 1), diff_length);
                    collector.add(self.offset.get_length_difference_index(j, i), -diff_length);

                    let diff_health = (healthes[i] - healthes[j]) as f32 / 100.0;
                    collector.add(self.offset.get_health_difference_index(i, j - 1), diff_health);
                    collector.add(self.offset.get_health_difference_index(j, i), -diff_health);
                }
            }
        });        
    }

    fn num_examples(&self) -> usize {
        self.examples_collector.borrow().len()
    }

    fn pop_examples(&mut self) -> Vec<Example> {
        self.examples_collector.borrow_mut().pop_examples()
    }
}
