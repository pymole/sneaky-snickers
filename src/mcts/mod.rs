cfg_if::cfg_if! {
    if #[cfg(feature = "par")] {
        pub mod parallel;
    } else {
        pub mod seq;
    }
}

pub mod config;
pub mod heuristics;
mod utils;
mod bandit;