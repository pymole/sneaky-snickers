use crate::engine::Movement;

use super::config::MCTSConfig;

pub trait MultiArmedBandit<T: MCTSConfig> {
    fn get_best_movement(&mut self, mcts_config: &T, node_visits: f32) -> usize;
    fn get_final_movement(&self, mcts_config: &T, node_visits: f32) -> Movement;
    fn backpropagate(&mut self, reward: f32, movement: usize);
    fn print_stats(&self, mcts_config: &T, node_visits: f32);
}
