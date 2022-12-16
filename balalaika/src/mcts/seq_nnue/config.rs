use crate::features::composite::CompositeFeatures;
use crate::nnue::Model;
use crate::mcts::utils::parse_env;

pub struct SequentialNNUEMCTSConfig {
    pub table_capacity: usize,
    pub rollout_cutoff: i32,
    pub draw_reward: f32,
    pub max_select_depth: usize,
    pub model: Model,
}

impl SequentialNNUEMCTSConfig {
    
    fn from_env() -> SequentialNNUEMCTSConfig {
        let feature_sets: String = parse_env("FEATURE_SETS").expect("Provide FEATURE_SETS");
        let feature_sets: Vec<String> = feature_sets.split(',').map(str::to_string).collect();
        let model = Model::new(
            tch::CModule::load("../analysis/weights/main.pt").unwrap(),
            CompositeFeatures::new(feature_sets),
        );

        let config = SequentialNNUEMCTSConfig {
            table_capacity:         parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            rollout_cutoff:         parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(0),
            draw_reward:            parse_env("MCTS_DRAW_REWARD").unwrap_or(0.01),
            max_select_depth:       parse_env("MCTS_SELECT_DEPTH").unwrap_or(50),
            model,
        };

        config
    }
}
