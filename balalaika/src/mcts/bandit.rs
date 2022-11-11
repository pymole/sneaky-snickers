use crate::engine::Movement;

use super::config::Config;

pub trait MultiArmedBandit<T: Config> {
    fn get_best_movement(&mut self, config: &T, node_visits: f32) -> usize;
    fn get_final_movement(&self, config: &T, node_visits: f32) -> Movement;
    fn backpropagate(&mut self, reward: f32, movement: usize);
    fn print_stats(&self, config: &T, node_visits: f32);
}
