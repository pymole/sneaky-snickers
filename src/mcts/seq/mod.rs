pub mod mcts;
pub mod config;

cfg_if::cfg_if! {
    if #[cfg(feature = "ts")] {
        mod thompson;
    } else {
        mod ucb;
    }
}
