use crate::engine::Movement;

use super::config::ParallelMCTSConfig;

#[derive(Clone, Debug)]
pub struct BUUCT {
    q: [f32; 4],
    variance: [f32; 4],
    rewards: [f32; 4],
    squared_rewards: [f32; 4],
    visits: [f32; 4],
    mask: [bool; 4],
    unobserved_samples: [f32; 4],
}

impl BUUCT {
    pub fn new(mask: [bool; 4]) -> BUUCT {
        BUUCT {
            q: [0.0; 4],
            rewards: [0.0; 4],
            visits: [0.0; 4],
            variance: [0.0; 4],
            squared_rewards: [0.0; 4],
            mask,
            unobserved_samples: [0.0; 4],
        }
    }
}

impl BUUCT {
    pub fn get_best_movement(&mut self, _config: &ParallelMCTSConfig, node_visits: f32, node_unobserved_samples: f32, iteration: usize) -> usize {
        let mut max_action = 0;
        let mut max_value = -1.0;
        let N = node_visits;
        let O = node_unobserved_samples;

        let N_add_O_ln = (N + O).ln();

        for action in (0..4).filter(|&m| self.mask[m]) {
            let n = self.visits[action];
            let o = self.unobserved_samples[action];

            let n_add_o = n + o;
            
            let ucb = if n_add_o > 0.0 {
                let variance_ucb = (self.variance[action] + (2.0 * N_add_O_ln / n_add_o).sqrt()).min(0.25);
                self.q[action] + (variance_ucb * N_add_O_ln / n_add_o).sqrt()
            } else {
                return action;
            };

            if ucb > max_value {
                max_action = action;
                max_value = ucb;
            }
        }

        max_action
    }

    pub fn get_final_movement(&self, _mcts_config: &ParallelMCTSConfig, _node_visits: f32) -> Movement {
        let (best_movement, _) = self.visits
            .iter()
            .enumerate()
            .max_by_key(|(_, &visits)| visits as usize)
            .unwrap_or((0, &0.0));

        Movement::from_usize(best_movement)
    }

    pub fn incomplete_update(&mut self, movement: usize) {
        self.unobserved_samples[movement] += 1.0;
    }

    pub fn backpropagate(&mut self, reward: f32, movement: usize) {        
        self.visits[movement] += 1.0;
        self.unobserved_samples[movement] -= 1.0;

        self.rewards[movement] += reward;
        self.squared_rewards[movement] += reward * reward;

        // Recalculate caches
        let q = self.rewards[movement] / self.visits[movement];
        self.q[movement] = q;

        let avg_squared_reward = self.squared_rewards[movement] / self.visits[movement];
        self.variance[movement] = avg_squared_reward - q * q;
    }

    pub fn print_stats(&self, _mcts_config: &ParallelMCTSConfig, _node_visits: f32) {
        info!("BU-UCT");
        info!("{:?}", self.visits);
        info!("{:?}", self.rewards);
        info!("{:?}", self.unobserved_samples);
        // info!("{:?}", self.unobserved_samples_avg);
        info!("{:?}", self.q);

        for action in 0..4 {
            let n_i = self.visits[action];
            if n_i > 0.0 {
                info!(
                    "[{}] - {}",
                    Movement::from_usize(action),
                    n_i,
                );
            } else {
                info!(" {}", Movement::from_usize(action));
            }
        }
    }
}
