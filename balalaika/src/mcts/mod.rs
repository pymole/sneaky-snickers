cfg_if::cfg_if! {
    if #[cfg(feature = "par")] {
        pub mod parallel;
    } else {
        pub mod seq;
    }
}

pub mod config;
pub mod heuristics;
pub mod search;
pub mod utils;
mod bandit;