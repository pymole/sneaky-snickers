use crate::engine::Movement;

pub trait MultiArmedBandit {
    fn get_best_movement(&mut self, config: &T, node_visits: f32) -> usize;
    fn get_final_movement(&self, config: &T, node_visits: f32) -> Movement;
    fn backpropagate(&mut self, reward: f32, movement: usize);
    fn print_stats(&self, config: &T, node_visits: f32);
}
