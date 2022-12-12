use crate::mcts::utils::parse_env;

#[derive(Clone, Copy, Debug)]
pub struct SequentialMCTSConfig {
    pub table_capacity: usize,
    pub rollout_cutoff: i32,
    pub draw_reward: f32,
    pub max_select_depth: usize,
}

impl SequentialMCTSConfig {
    pub fn from_env() -> SequentialMCTSConfig {
        let config = SequentialMCTSConfig {
            table_capacity: parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            rollout_cutoff: parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(0),
            draw_reward:    parse_env("MCTS_DRAW_REWARD").unwrap_or(0.01),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
        };

        config
    }
}
