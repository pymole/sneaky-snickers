use crate::mcts::bandit::MultiArmedBandit;
use crate::engine::Movement;

use super::config::SequentialMCTSConfig;

#[derive(Clone, Debug)]
pub struct UCB {
    rewards: [f32; 4],
    squared_rewards: [f32; 4],
    variance: [f32; 4],
    visits: [f32; 4],
    pub mask: [bool; 4],
    q: [f32; 4],
}

impl UCB {
    pub fn new(mask: [bool; 4]) -> UCB {
        UCB {
            rewards: [0f32; 4],
            squared_rewards: [0f32; 4],
            visits: [0f32; 4],
            q: [0f32; 4],
            variance: [0f32; 4],
            mask: mask,
        }
    }
}

impl MultiArmedBandit<SequentialMCTSConfig> for UCB {
    fn get_best_movement(&mut self, _mcts_config: &SequentialMCTSConfig, n_ln: f32) -> usize {
        let mut max_ucb_action = 0;
        let mut max_ucb = -1.0;

        for action in (0..4).filter(|&m| self.mask[m]) {
            let n_i = self.visits[action];

            if n_i == 0.0 {
                return action;
            }

            let ucb = self.movement_value(_mcts_config, action, n_ln);

            if ucb > max_ucb {
                max_ucb_action = action;
                max_ucb = ucb;
            }
        }

        max_ucb_action
    }

    fn get_final_movement(&self, _mcts_config: &SequentialMCTSConfig, _node_visits: f32) -> Movement {
        let (best_movement, _) = self.visits
            .iter()
            .enumerate()
            .max_by_key(|(_, &visits)| visits as usize)
            .unwrap_or((0, &0.0));

        Movement::from_usize(best_movement)
    }

    fn movement_value(&self, _mcts_config: &SequentialMCTSConfig, movement: usize, n_ln: f32) -> f32 {
        let n_i = self.visits[movement];
        let variance_ucb = (self.variance[movement] + (2.0 * n_ln / n_i).sqrt()).min(0.25);
        self.q[movement] + (variance_ucb * n_ln / n_i).sqrt()
    }

    fn backpropagate(&mut self, reward: f32, movement: usize) {
        self.rewards[movement] += reward;
        self.squared_rewards[movement] += reward * reward;
        self.visits[movement] += 1.0;
        let q = self.rewards[movement] / self.visits[movement];
        self.q[movement] = q;

        let avg_squared_reward = self.squared_rewards[movement] / self.visits[movement];
        self.variance[movement] = avg_squared_reward - q * q;
    }

    fn print_stats(&self, _mcts_config: &SequentialMCTSConfig, n_ln: f32) {
        info!("UCB");
        // let selecte_move = get_best_movement_from_movement_visits(ucb_instance.visits);
        for action in 0..4 {
            let n_i = self.visits[action];
            if n_i > 0.0 {
                // copy-paste
                let variance = self.variance[action];
                let variance_ucb = (variance + (2.0 * n_ln / n_i).sqrt()).min(0.25);
                
                let explore = (variance_ucb * n_ln / n_i).sqrt();

                // let (selected_move, _) = self.get_best_movement(mcts_config, node_visits);

                info!(
                    "[{}] - {:.4}  {:.4}   {}",
                    Movement::from_usize(action),
                    self.q[action],
                    explore,
                    n_i,
                );
            } else {
                info!(" {}", Movement::from_usize(action));
            }
        }
    }
}
