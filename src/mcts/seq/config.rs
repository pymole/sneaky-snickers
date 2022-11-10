use std::time::Duration;
use crate::mcts::config::{Config, parse_env};

#[derive(Clone, Copy, Debug)]
pub struct SequentialMCTSConfig {
    pub table_capacity: usize,
    pub iterations: Option<usize>,
    pub search_time: Option<Duration>,
    pub rollout_cutoff: i32,
    pub rollout_beta: f32,
    pub draw_reward: f32,
    pub max_select_depth: usize,
}

impl Config for SequentialMCTSConfig {
    fn from_env() -> SequentialMCTSConfig {
        let config = SequentialMCTSConfig {
            table_capacity: parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            iterations:     parse_env("MCTS_ITERATIONS"),
            search_time:    parse_env("MCTS_SEARCH_TIME").map(Duration::from_millis),
            rollout_cutoff: parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(0),
            rollout_beta:   parse_env("MCTS_ROLLOUT_BETA").unwrap_or(3.0),
            draw_reward:    parse_env("MCTS_DRAW_REWARD").unwrap_or(0.01),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
        };

        config
    }

    fn get_search_iterations(&self) -> Option<usize> {
        self.iterations
    }

    fn get_search_time(&self) -> Option<Duration> {
        self.search_time
    }
}
