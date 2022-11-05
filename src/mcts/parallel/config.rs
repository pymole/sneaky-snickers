use std::time::Duration;

use num_cpus::get as num_cpus;

use crate::mcts::config::{MCTSConfig, parse_env};

#[derive(Clone, Copy, Debug)]
pub struct ParallelMCTSConfig {
    pub table_capacity: usize,
    pub iterations: Option<u32>,
    pub search_time: Option<Duration>,
    pub rollout_cutoff: i32,
    pub rollout_beta: f32,
    pub draw_reward: f32,
    pub workers: usize,
    pub max_select_depth: usize,
}

impl MCTSConfig for ParallelMCTSConfig {
    fn from_env() -> ParallelMCTSConfig {
        let config = ParallelMCTSConfig {
            table_capacity:         parse_env("MCTS_TABLE_CAPACITY").unwrap_or(400000),
            iterations:             parse_env("MCTS_ITERATIONS"),
            search_time:            Some(Duration::from_millis(parse_env("MCTS_SEARCH_TIME").unwrap_or(300))),
            rollout_cutoff:         parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(0),
            rollout_beta:           parse_env("MCTS_ROLLOUT_BETA").unwrap_or(3.0),
            draw_reward:            parse_env("MCTS_DRAW_REWARD").unwrap_or(0.01),
            workers:                parse_env("MCTS_WORKERS").unwrap_or(num_cpus()),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
        };

        config
    }
}
