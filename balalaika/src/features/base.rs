use std::{rc::Rc, cell::RefCell, mem, collections::HashMap};

use crate::{
    game::{
        self,
        Board,
        Snake,
        GridPoint,
    },
    zobrist::{body_direction, BodyDirections},
    engine::safe_zone_shrinker::SHRINK_EVERY
};

use super::{
    collector::{
        IndexType,
        SparseCollector,
        FeaturesHandler,
        get_point_index,
        ALL_PLAYERS_GRIDS_SIZE,
        MAX_SNAKE_COUNT,
        SIZE,
        ExamplesHandler, ValueType
    },
    examples::ExamplesCollector, Example
};


const MAX_BODIES_ON_TAIL: IndexType = 3;
const HEALTH_BARS: IndexType = 101;

pub struct BaseFeatures {
    collector: SparseCollector,
    offset: BaseOffset,
}

impl BaseFeatures {
    pub fn new(offset: BaseOffset) -> BaseFeatures {
        BaseFeatures {
            collector: SparseCollector::new(),
            offset,
        }
    }
}

impl FeaturesHandler for BaseFeatures {
    fn pre(&mut self, _board: &Board) {}

    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, snake: &Snake, _alive_index: usize, snake_index: usize, _board: &Board) {
        // (owner[snakes], health[101])
        let health = snake.health as IndexType;
        self.collector.add(self.offset.get_health_index(snake_index, health), 1.0);

        // (owner[snakes], head[grid])
        let head = snake.head();
        self.collector.add(self.offset.get_head_index(snake_index, head), 1.0);

        // (owner[snakes], tail[grid])
        self.collector.add(self.offset.get_tail_index(snake_index, snake.tail()), 1.0);

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;
                let direction = body_direction(back, front);
                self.collector.add(self.offset.get_body_part_index(snake_index, back, direction), 1.0);
                break;
            }
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            self.collector.add(self.offset.get_body_part_index(snake_index, front, direction), 1.0);
        }
        self.collector.add(self.offset.get_body_part_index(snake_index, head, BodyDirections::Still), 1.0);

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = (snake.body.len() - nearest_to_head_body_on_tail_index) as IndexType;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        
        self.collector.add(self.offset.get_bodies_on_tail_index(snake_index, bodies_on_tail), 1.0);
    }
    
    fn food(&mut self, pos: GridPoint) {
        // (food[grid], )
        self.collector.add(self.offset.get_food_index(pos), 1.0);
    }

    fn hazard(&mut self, pos: GridPoint) {
        // (hazard[grid], )
        self.collector.add(self.offset.get_hazard_index(pos), 1.0);
    }

    fn post(&mut self, board: &Board) {
        let turns_left_before_shrink = (board.turn % SHRINK_EVERY) as f32 / SHRINK_EVERY as f32;
        self.collector.add(self.offset.get_turn_left_before_shrink_index(), turns_left_before_shrink);
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
const HEADS_AT: IndexType = 0;
const TAILS_AT: IndexType = ALL_PLAYERS_GRIDS_SIZE;
const BODY_PARTS_AT: IndexType = TAILS_AT + ALL_PLAYERS_GRIDS_SIZE;
const BODIES_ON_TAIL_AT: IndexType = BODY_PARTS_AT + ALL_PLAYERS_GRIDS_SIZE * 5;
const FOOD_AT: IndexType = BODIES_ON_TAIL_AT + MAX_BODIES_ON_TAIL * MAX_SNAKE_COUNT;
const HAZARD_AT: IndexType = FOOD_AT + SIZE;
const HEALTH_AT: IndexType = HAZARD_AT + SIZE;
const TURNS_LEFT_BEFORE_SHRINK_AT: IndexType = HEALTH_AT + MAX_SNAKE_COUNT * HEALTH_BARS;
pub const NUM_FEATURES: IndexType = TURNS_LEFT_BEFORE_SHRINK_AT + 1;


// Offset index calculations
fn calculate_snake_heads_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut owner_heads_at = [0; game::MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        owner_heads_at[i as usize] = offset + HEADS_AT + i * SIZE;
    }
    owner_heads_at
}

fn calculate_snake_tails_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut owner_tails_at = [0; game::MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        owner_tails_at[i as usize] = offset + TAILS_AT + i * SIZE;
    }
    owner_tails_at
}

fn calculate_snake_body_parts_at(offset: IndexType) -> [[IndexType; 5]; game::MAX_SNAKE_COUNT] {
    // TODO: split direction and body
    let mut owner_body_parts_at = [[0; 5]; game::MAX_SNAKE_COUNT];
    let mut owner_offset;
    for i in 0..MAX_SNAKE_COUNT {
        owner_offset = BODY_PARTS_AT + i * SIZE * 5;
        for d in 0..5 {
            owner_body_parts_at[i as usize][d as usize] = offset + owner_offset + d * SIZE;
        }
    }
    owner_body_parts_at
}

fn calculate_snake_bodies_on_tail_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut owner_bodies_on_tail_at = [0; game::MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        owner_bodies_on_tail_at[i as usize] = offset + BODIES_ON_TAIL_AT + i * MAX_BODIES_ON_TAIL;
    }
    owner_bodies_on_tail_at
}

fn calculate_hazard_at(offset: IndexType) -> IndexType {
    offset + HAZARD_AT
}

fn calculate_food_at(offset: IndexType) -> IndexType {
    offset + FOOD_AT
}

fn calculate_health_at(offset: IndexType) -> [IndexType; game::MAX_SNAKE_COUNT] {
    let mut owner_on_health_at = [0; game::MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        owner_on_health_at[i as usize] = offset + HEALTH_AT + i * HEALTH_BARS;
    }
    owner_on_health_at
}

fn calculate_turns_left_before_shrink_at(offset: IndexType) -> IndexType {
    offset + TURNS_LEFT_BEFORE_SHRINK_AT
}

#[derive(Debug)]
pub struct BaseOffset {
    heads_at: [IndexType; game::MAX_SNAKE_COUNT],
    tails_at: [IndexType; game::MAX_SNAKE_COUNT],
    body_parts_at: [[IndexType; 5]; game::MAX_SNAKE_COUNT],
    bodies_on_tail_at: [IndexType; game::MAX_SNAKE_COUNT],
    health_at: [IndexType; game::MAX_SNAKE_COUNT],
    food_at: IndexType,
    hazard_at: IndexType,
    turns_left_before_shrink_at: IndexType,
}

impl BaseOffset {
    pub fn new(offset: IndexType) -> BaseOffset {
        BaseOffset {
            heads_at: calculate_snake_heads_at(offset),
            tails_at: calculate_snake_tails_at(offset),
            body_parts_at: calculate_snake_body_parts_at(offset),
            bodies_on_tail_at: calculate_snake_bodies_on_tail_at(offset),
            health_at: calculate_health_at(offset),
            food_at: calculate_food_at(offset),
            hazard_at: calculate_hazard_at(offset),
            turns_left_before_shrink_at: calculate_turns_left_before_shrink_at(offset),
        }
    }

    fn get_health_index(&self, owner: usize, health: IndexType) -> IndexType {
        self.health_at[owner] + health
    }
    
    fn get_head_index(&self, owner: usize, head: GridPoint) -> IndexType {
        self.heads_at[owner] + get_point_index(head)
    }
    
    fn get_tail_index(&self, owner: usize, tail: GridPoint) -> IndexType {
        self.tails_at[owner] + get_point_index(tail)
    }
    
    fn get_body_part_index(&self, owner: usize, body: GridPoint, body_direction: BodyDirections) -> IndexType {
        self.body_parts_at[owner][body_direction as usize] + get_point_index(body)
    }
    
    fn get_bodies_on_tail_index(&self, owner: usize, bodies_on_tail: IndexType) -> IndexType {
        self.bodies_on_tail_at[owner] + bodies_on_tail - 1
    }
    
    fn get_food_index(&self, food: GridPoint) -> IndexType {
        self.food_at + get_point_index(food)
    }
    
    fn get_hazard_index(&self, hazard: GridPoint) -> IndexType {
        self.hazard_at + get_point_index(hazard)
    }
    
    fn get_turn_left_before_shrink_index(&self) -> IndexType {
        self.turns_left_before_shrink_at
    }
}

pub fn inspect() -> HashMap<&'static str, IndexType> {
    let mut data = HashMap::new();
    data.insert("heads_at", HEADS_AT);
    data.insert("tails_at", TAILS_AT);
    data.insert("body_parts_at", BODY_PARTS_AT);
    data.insert("bodies_on_tail_at", BODIES_ON_TAIL_AT);
    data.insert("health_at", HEALTH_AT);
    data.insert("food_at", FOOD_AT);
    data.insert("hazard_at", HAZARD_AT);
    data.insert("turns_left_before_shrink_at", TURNS_LEFT_BEFORE_SHRINK_AT);
    data
}


/// Rotates, flips and permutates snakes
/// Index pattern:
/// | rot                       | rot                       |
/// | flip        | flip        | flip        | flip        |
/// | perm | perm | perm | perm | perm | perm | perm | perm |
/// Rotations: 4
/// Flips on rotation: 3
/// Snake positions permutations: A(n=MAX_SNAKES_COUNT, k=alive_snakes_count)


pub struct BaseExamples {
    examples_collector: Rc<RefCell<ExamplesCollector>>,
    offset: BaseOffset,
}

impl BaseExamples {
    pub fn new(examples_collector: Rc<RefCell<ExamplesCollector>>, offset: BaseOffset) -> BaseExamples {
        BaseExamples {
            examples_collector,
            offset,
        }
    }
}

impl ExamplesHandler for BaseExamples {
    fn pre(&mut self, _board: &Board) {}
    
    fn snake(&mut self, _snake: &Snake, _snake_index: usize, _board: &Board) {}

    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, _snake_index: usize, _board: &Board) {
        // (owner[snakes], health[101])
        let health = snake.health as IndexType;

        let mut examples_collector = self.examples_collector.borrow_mut();

        examples_collector.alive_snake_parameter_filler(
            alive_index,
            |collector, owner| {
                collector.add(self.offset.get_health_index(owner, health), 1.0);
            },
        );
        
        // (owner[snakes], head[grid])
        let head = snake.head();
        examples_collector.alive_snake_point_filler(
            head,
            alive_index,
            |collector, owner, point| {
                collector.add(self.offset.get_head_index(owner, point), 1.0);
            },
        );

        // (owner[snakes], tail[grid])
        examples_collector.alive_snake_point_filler(
            snake.tail(),
            alive_index,
            |collector, owner, point| {
                collector.add(self.offset.get_tail_index(owner, point), 1.0);
            },
        );

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;

                let direction = body_direction(back, front);
                examples_collector.alive_snake_body_part_filler(
                    back,
                    direction,
                    alive_index,
                    |collector, owner, point, dir| {
                        collector.add(self.offset.get_body_part_index(owner, point, dir), 1.0);
                    }
                );
                break;
            }
            
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            examples_collector.alive_snake_body_part_filler(
                front,
                direction,
                alive_index,
                |collector, owner, point, dir| {
                    collector.add(self.offset.get_body_part_index(owner, point, dir), 1.0);
                }
            );
        }
        examples_collector.alive_snake_body_part_filler(
            head,
            BodyDirections::Still,
            alive_index,
            |collector, owner, point, dir| {
                collector.add(self.offset.get_body_part_index(owner, point, dir), 1.0);
            }
        );

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = (snake.body.len() - nearest_to_head_body_on_tail_index) as IndexType;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        
        examples_collector.alive_snake_parameter_filler(
            alive_index,
            |collector, owner| {
                collector.add(self.offset.get_bodies_on_tail_index(owner, bodies_on_tail), 1.0);
            },
        );
    }

    fn food(&mut self, pos: GridPoint) {
        self.examples_collector.borrow_mut().grid_point_filler(
            pos,
            |collector, point| {
                collector.add(self.offset.get_food_index(point), 1.0);
            }
        )
    }

    fn hazard(&mut self, pos: GridPoint) {
        self.examples_collector.borrow_mut().grid_point_filler(
            pos,
            |collector, point| {
                collector.add(self.offset.get_hazard_index(point), 1.0);
            }
        )
    }

    fn post(&mut self, board: &Board) {
        let turns_left_before_shrink = (board.turn % SHRINK_EVERY) as f32 / SHRINK_EVERY as f32 ;
        self.examples_collector.borrow_mut().parameter(
            self.offset.get_turn_left_before_shrink_index(),
            turns_left_before_shrink,
        );
    }

    fn num_examples(&self) -> usize {
        self.examples_collector.borrow().len()
    }

    fn pop_examples(&mut self) -> Vec<Example> {
        self.examples_collector.borrow_mut().pop_examples()
    }
}
