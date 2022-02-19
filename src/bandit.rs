use crate::mcts::MCTSConfig;
use crate::engine::Movement;
use crate::game::Board;

pub trait MultiArmedBandit {
    fn get_best_movement(&mut self, mcts_config: &MCTSConfig, node_visits: u32) -> usize;
    fn get_final_movement(&self, mcts_config: &MCTSConfig, node_visits: u32) -> Movement;
    fn backpropagate(&mut self, reward: f32, movement: usize);
    fn print_stats(&self, mcts_config: &MCTSConfig, node_visits: u32);
}
