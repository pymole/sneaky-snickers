use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::game::{Board, Snake, GridPoint};
use super::flood_fill::{FloodFillOffset, FloodFillFeatures, self, FloodFillExamples};
use super::{Example, snakes_metrics};
use super::base::{self, BaseExamples, BaseFeatures, BaseOffset};
use super::collector::{FeaturesHandler, IndexType, ExamplesHandler, ValueType};
use super::examples::{ExamplesCollector, ExamplesContext};
use super::global_metrics::{self, GlobalMetricsExamples, GlobalMetricsFeatures, GlobalMetricsOffset};
use super::snakes_metrics::{SnakeMetricsFeatures, SnakeMetricsOffset, SnakeMetricsExamples};


pub struct CompositeFeatures {
    handlers: Vec<Box<dyn FeaturesHandler>>,
    num_features: IndexType,
}

impl CompositeFeatures {
    pub fn new(feature_tags: Vec<String>) -> CompositeFeatures {
        let mut handlers = Vec::new();
        let mut offset = 0;
        for tag in feature_tags {
            let handler: Box<dyn FeaturesHandler> = match tag.as_str() {
                "base" => {
                    let handler = Box::new(BaseFeatures::new(BaseOffset::new(offset)));
                    offset += base::NUM_FEATURES;
                    handler
                },
                "global_metrics" => {
                    let handler = Box::new(GlobalMetricsFeatures::new(GlobalMetricsOffset::new(offset)));
                    offset += global_metrics::NUM_FEATURES;
                    handler
                },
                "snakes_metrics" => {
                    let handler = Box::new(SnakeMetricsFeatures::new(SnakeMetricsOffset::new(offset)));
                    offset += snakes_metrics::NUM_FEATURES;
                    handler
                },
                "flood_fill" => {
                    let handler = Box::new(FloodFillFeatures::new(FloodFillOffset::new(offset)));
                    offset += flood_fill::NUM_FEATURES;
                    handler
                },
                _ => panic!("Wrong feature set"),
            };
            handlers.push(handler);
        }

        CompositeFeatures {
            handlers,
            num_features: offset,
        }
    }
}

impl FeaturesHandler for CompositeFeatures {
    fn pre(&mut self, board: &Board) {
        for handler in &mut self.handlers {
            handler.pre(board);
        }
    }
    
    fn snake(&mut self, snake: &Snake, snake_index: usize, board: &Board) {
        for handler in &mut self.handlers {
            handler.snake(snake, snake_index, board);
        }
    }

    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, snake_index: usize, board: &Board) {
        for handler in &mut self.handlers {
            handler.alive_snake(snake, alive_index, snake_index, board);
        }
    }

    fn food(&mut self, pos: GridPoint) {
        for handler in &mut self.handlers {
            handler.food(pos);
        }
    }

    fn hazard(&mut self, pos: GridPoint) {
        for handler in &mut self.handlers {
            handler.hazard(pos);
        }
    }

    fn post(&mut self, board: &Board) {
        for handler in &mut self.handlers {
            handler.post(board);
        }
    }

    fn num_features(&self) -> IndexType {
        self.num_features
    }

    fn pop_features(&mut self) -> (Vec<IndexType>, Vec<ValueType>) {
        let mut indices = Vec::with_capacity(self.num_features as usize);
        let mut values = Vec::with_capacity(self.num_features as usize);
        for handler in &mut self.handlers {
            let (i, v) = handler.pop_features();
            indices.extend(i);
            values.extend(v);
        }
        (indices, values)
    }
}

// Examples

pub struct CompositeExamples {
    handlers: Vec<Box<dyn ExamplesHandler>>,
    pub examples_collector: Rc<RefCell<ExamplesCollector>>,
}

impl CompositeExamples {
    pub fn new(feature_tags: Vec<String>, context: ExamplesContext) -> CompositeExamples {
        let mut handlers = Vec::new();
        let mut offset = 0;
        
        let examples_collector = Rc::new(RefCell::new(ExamplesCollector::new(context)));
        for tag in feature_tags {
            let collector = Rc::clone(&examples_collector);
            let handler: Box<dyn ExamplesHandler> = match tag.as_str() {
                "base" => {
                    let handler = Box::new(BaseExamples::new(collector, BaseOffset::new(offset)));
                    offset += base::NUM_FEATURES;
                    handler
                },
                "global_metrics" => {
                    let handler = Box::new(GlobalMetricsExamples::new(collector, GlobalMetricsOffset::new(offset)));
                    offset += global_metrics::NUM_FEATURES;
                    handler
                },
                "snakes_metrics" => {
                    let handler = Box::new(SnakeMetricsExamples::new(collector, SnakeMetricsOffset::new(offset)));
                    offset += snakes_metrics::NUM_FEATURES;
                    handler
                },
                "flood_fill" => {
                    let handler = Box::new(FloodFillExamples::new(collector, FloodFillOffset::new(offset)));
                    offset += flood_fill::NUM_FEATURES;
                    handler
                },
                _ => panic!("Wrong feature set"),
            };
            handlers.push(handler);
        }

        CompositeExamples {
            handlers,
            examples_collector,
        }
    }
}

impl ExamplesHandler for CompositeExamples {
    fn pre(&mut self, board: &Board) {
        for handler in &mut self.handlers {
            handler.pre(board);
        }
    }

    fn snake(&mut self, snake: &Snake, snake_index: usize, board: &Board) {
        for handler in &mut self.handlers {
            handler.snake(snake, snake_index, board);
        }
    }

    fn alive_snake(&mut self, snake: &Snake, alive_index: usize, snake_index: usize, board: &Board) {
        for handler in &mut self.handlers {
            handler.alive_snake(snake, alive_index, snake_index, board);
        }
    }

    fn food(&mut self, pos: GridPoint) {
        for handler in &mut self.handlers {
            handler.food(pos);
        }
    }

    fn hazard(&mut self, pos: GridPoint) {
        for handler in &mut self.handlers {
            handler.hazard(pos);
        }
    }

    fn post(&mut self, board: &Board) {
        for handler in &mut self.handlers {
            handler.post(board);
        }
    }

    fn num_examples(&self) -> usize {
        self.examples_collector.borrow().len()
    }

    fn pop_examples(&mut self) -> Vec<Example> {
        self.examples_collector.borrow_mut().pop_examples()
    }
}


pub fn feature_set_sizes() -> HashMap<&'static str, IndexType> {
    let mut sizes = HashMap::new();
    sizes.insert("base", base::NUM_FEATURES);
    sizes.insert("global_metrics", global_metrics::NUM_FEATURES);
    sizes.insert("snakes_metrics", snakes_metrics::NUM_FEATURES);
    sizes.insert("flood_fill", flood_fill::NUM_FEATURES);
    sizes
}