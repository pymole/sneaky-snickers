use crate::engine::Movement;

use super::config::ParallelMCTSConfig;

#[derive(Clone, Debug)]
pub struct BUUCT {
    q: [f32; 4],
    rewards: [f32; 4],
    visits: [f32; 4],
    mask: [bool; 4],
    unobserved_samples: [f32; 4],
    unobserved_samples_avg: [f32; 4],
}

impl BUUCT {
    pub fn new(mask: [bool; 4]) -> BUUCT {
        BUUCT {
            q: [0.0; 4],
            rewards: [0.0; 4],
            visits: [0.0; 4],
            mask,
            unobserved_samples: [0.0; 4],
            unobserved_samples_avg: [0.0; 4],
        }
    }
}

impl BUUCT {
    pub fn get_best_movement(&mut self, config: &ParallelMCTSConfig, node_visits: f32, node_unobserved_samples: f32, iteration: usize) -> usize {
        let mut max_action = 0;
        let mut max_value = -1.0;
        let N = node_visits;
        let O = node_unobserved_samples;

        for action in (0..4).filter(|&m| self.mask[m]) {
            let n = self.visits[action];
            let o = self.unobserved_samples[action];

            let n_add_o = n + o;

            let bu_uct = if n_add_o > 0.0 {
                // let surrogate_gap = self.unobserved_samples_avg[action];
                
                // if surrogate_gap < config.m_max * config.simulation_workers as f32 {
                let q = self.q[action];
                q + config.c * (2.0 * (N + O).ln() / n_add_o).sqrt()
                // } else {
                //     -f32::INFINITY
                // }
            } else {
                f32::INFINITY
            };

            if bu_uct > max_value {
                max_action = action;
                max_value = bu_uct;
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

        let n = self.visits[movement];
        let o = self.unobserved_samples[movement];
        let n_add_o = n + o;
        let o_avg = self.unobserved_samples_avg[movement];

        self.unobserved_samples_avg[movement] = (n_add_o - 1.0) / n_add_o * o_avg + o / n_add_o;
    }

    pub fn backpropagate(&mut self, reward: f32, movement: usize) {        
        self.visits[movement] += 1.0;
        self.unobserved_samples[movement] -= 1.0;
        self.rewards[movement] += reward;

        let n = self.visits[movement];
        let r_cummulative = self.rewards[movement];
        // let q = self.q[movement];

        self.q[movement] = r_cummulative / n;
    }

    pub fn print_stats(&self, _mcts_config: &ParallelMCTSConfig, _node_visits: f32) {
        info!("BU-UCT");
        info!("{:?}", self.visits);
        info!("{:?}", self.rewards);
        info!("{:?}", self.unobserved_samples);
        info!("{:?}", self.unobserved_samples_avg);
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
