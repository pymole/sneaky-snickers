use std::time::Duration;

use crate::{game::Board, api::objects::Movement};

use super::config::Config;

pub trait Search<C: Config> {
    fn search(&mut self, board: &Board, iterations_count: usize);
 
    fn search_with_time(&mut self, board: &Board, duration: Duration);

    fn get_final_movement(&self, board: &Board, agent_index: usize) -> Movement;

    fn get_config(&self) -> &C;

    fn shutdown(&self);
}
