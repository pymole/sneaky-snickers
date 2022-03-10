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
    pub simulation_workers: usize,
    pub m_max: f32,
    pub c: f32,
    pub max_select_depth: usize,
}

impl MCTSConfig for ParallelMCTSConfig {
    fn from_env() -> ParallelMCTSConfig {
        const WIN_POINTS : f32 = 15.0;
        const DRAW_POINTS : f32 = 0.0;
        const LOSS_POINTS : f32 = -3.0;
        const NORMALIZED_DRAW_REWARD : f32 = (DRAW_POINTS - LOSS_POINTS) / (WIN_POINTS - LOSS_POINTS);

        let config = ParallelMCTSConfig {
            table_capacity:         parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            iterations:             parse_env("MCTS_ITERATIONS"),
            search_time:            Some(Duration::from_millis(parse_env("MCTS_SEARCH_TIME").unwrap_or(300))),
            rollout_cutoff:         parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(21),
            rollout_beta:           parse_env("MCTS_ROLLOUT_BETA").unwrap_or(3.0),
            draw_reward:            parse_env("MCTS_DRAW_REWARD").unwrap_or(NORMALIZED_DRAW_REWARD),
            simulation_workers:     parse_env("MCTS_SIMULATION_WORKERS").unwrap_or(num_cpus()),
            m_max:                  parse_env("MCTS_M_MAX").unwrap_or(0.8),
            c:                      parse_env("MCTS_C").unwrap_or(0.8),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
        };

        assert!(config.rollout_cutoff >= 1);
        assert!(config.m_max >= 0.0 && config.m_max <= 1.0);

        config
    }
}
