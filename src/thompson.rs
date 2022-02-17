use crate::bandit::MultiArmedBandit;
use crate::mcts::MCTSConfig;
use crate::engine::Movement;
use rand::distributions::Distribution;
use statrs::distribution::Beta;


#[derive(Clone, Debug)]
pub struct ThompsonSampling {
    beta_distributions: [Beta; 4],
    mask: [bool; 4],
}

impl ThompsonSampling {
    pub fn new(mask: [bool; 4]) -> ThompsonSampling {
        ThompsonSampling {
            beta_distributions: [Beta::new(1.0, 1.0).expect("Wrong a or b"); 4],
            mask: mask,
        }
    }
}

fn _get_best_movement(ts: &ThompsonSampling) -> usize {
    let mut max_action = 0;
    let mut max_v = -1.0;

    for action in (0..4).filter(|&m| ts.mask[m]) {
        let beta = ts.beta_distributions[action];
        let v = beta.sample(&mut rand::thread_rng());

        if v > max_v {
            max_action = action;
            max_v = v;
        }
    }

    max_action
}

impl MultiArmedBandit for ThompsonSampling {
    fn get_best_movement(&mut self, _mcts_config: &MCTSConfig, _node_visits: u32) -> usize {
        _get_best_movement(self)
    }

    fn get_final_movement(&self, _mcts_config: &MCTSConfig, _node_visits: u32) -> Movement {
        let best_movement = _get_best_movement(self);
        Movement::from_usize(best_movement)
    }

    fn backpropagate(&mut self, reward: f32, movement: usize) {
        let beta = self.beta_distributions[movement];
        let win = ((reward == 1.0) as usize) as f64;
        self.beta_distributions[movement] = Beta::new(
            beta.shape_a() + win,
            beta.shape_b() + (1.0 - win)
        ).expect("Wrong a and b");
    }

    fn print_stats(&self, _mcts_config: &MCTSConfig, _node_visits: u32) {
        info!("Thompson Sampling");
    }
}
