mod mcts;
mod config;

pub use mcts::SequentialMCTS;
pub use config::SequentialMCTSConfig;

cfg_if::cfg_if! {
    if #[cfg(feature = "ts")] {
        mod thompson;
    } else {
        mod ucb;
    }
}
