use std::time::Duration;

use crate::{game::Board, api::objects::Movement};

pub trait Search {
    fn search(&mut self, board: &Board, iterations_count: usize);
 
    fn search_with_time(&mut self, board: &Board, duration: Duration) -> usize;

    fn get_final_movement(&self, board: &Board, agent_index: usize) -> Movement;

    fn shutdown(&self);
}
