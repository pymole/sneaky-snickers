use crate::mcts::bandit::MultiArmedBandit;
use crate::engine::Movement;

use super::config::ParallelMCTSConfig;

#[derive(Clone, Debug)]
pub struct BUUCT {
    rewards: [f32; 4],
    squared_rewards: [f32; 4],
    visits: [u32; 4],
    mask: [bool; 4],
}

impl BUUCT {
    pub fn new(mask: [bool; 4]) -> BUUCT {
        BUUCT {
            rewards: [0f32; 4],
            squared_rewards: [0f32; 4],
            visits: [0u32; 4],
            mask: mask,
        }
    }
}

impl MultiArmedBandit<ParallelMCTSConfig> for BUUCT {
    fn get_best_movement(&mut self, _mcts_config: &ParallelMCTSConfig, node_visits: u32) -> usize {
        let mut max_ucb_action = 0;
        let mut max_ucb = -1.0;

        for action in (0..4).filter(|&m| self.mask[m]) {
            let n = node_visits as f32;
            let n_i = self.visits[action] as f32;

            let ucb = if n_i > 0.0 {
                let avg_reward = self.rewards[action] / n_i;
                let avg_squared_reward = self.squared_rewards[action] / n_i;
                let variance = avg_squared_reward - avg_reward * avg_reward;
                let variance_ucb = (variance + (2.0 * n.ln() / n_i).sqrt()).min(0.25);

                avg_reward + (variance_ucb * n.ln() / n_i).sqrt()
            } else {
                f32::INFINITY
            };

            if ucb > max_ucb {
                max_ucb_action = action;
                max_ucb = ucb;
            }
        }

        max_ucb_action
    }

    fn get_final_movement(&self, _mcts_config: &ParallelMCTSConfig, _node_visits: u32) -> Movement {
        let (best_movement, _) = self.visits
            .iter()
            .enumerate()
            .max_by_key(|(_, &visits)| visits)
            .unwrap_or((0, &0));

        Movement::from_usize(best_movement)
    }

    fn backpropagate(&mut self, reward: f32, movement: usize) {
        self.rewards[movement] += reward;
        self.squared_rewards[movement] += reward * reward;
        self.visits[movement] += 1;
    }

    fn print_stats(&self, _mcts_config: &ParallelMCTSConfig, node_visits: u32) {
        let n = node_visits as f32;

        info!("UCB");
        // let selecte_move = get_best_movement_from_movement_visits(ucb_instance.visits);
        for action in 0..4 {
            let n_i = self.visits[action] as f32;
            if n_i > 0.0 {
                // copy-paste
                let avg_reward = self.rewards[action] / n_i;
                let avg_squared_reward = self.squared_rewards[action] / n_i;
                let variance = avg_squared_reward - avg_reward * avg_reward;
                let variance_ucb = (variance + (2.0 * n.ln() / n_i).sqrt()).min(0.25);

                let explore = (variance_ucb * n.ln() / n_i).sqrt();

                // let (selected_move, _) = self.get_best_movement(mcts_config, node_visits);

                info!(
                    "[{}] - {:.4}  {:.4}   {}",
                    Movement::from_usize(action),
                    avg_reward,
                    explore,
                    n_i,
                );
            } else {
                info!(" {}", Movement::from_usize(action));
            }
        }
    }
}
