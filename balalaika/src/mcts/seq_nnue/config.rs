use tch::CModule;

use crate::mcts::utils::parse_env;

#[derive(Debug)]
pub struct SequentialNNUEMCTSConfig {
    pub table_capacity: usize,
    pub rollout_cutoff: i32,
    pub draw_reward: f32,
    pub max_select_depth: usize,
    pub model: CModule,
}

impl SequentialNNUEMCTSConfig {
    
    fn from_env() -> SequentialNNUEMCTSConfig {
        let config = SequentialNNUEMCTSConfig {
            table_capacity:         parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            rollout_cutoff:         parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(0),
            draw_reward:            parse_env("MCTS_DRAW_REWARD").unwrap_or(0.01),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
            model:                  CModule::load("MCTS_MODEL_PATH").unwrap(),
        };

        config
    }
}
