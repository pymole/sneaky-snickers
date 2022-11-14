use std::env;
use std::str::FromStr;
use std::time::Duration;


pub trait Config {
    fn from_env() -> Self;
    fn get_search_time(&self) -> Option<Duration>;
    fn get_search_iterations(&self) -> Option<usize>;
}

pub fn parse_env<Value>(key: &str) -> Option<Value>
where
    Value: FromStr,
    <Value as FromStr>::Err: std::fmt::Debug
{
    env::var(key).map(|s| s.parse().expect("MCTSConfig: can't parse env variable")).ok()
}