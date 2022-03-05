use std::env;
use std::str::FromStr;

pub trait MCTSConfig {
    fn from_env() -> Self;
}

pub fn parse_env<Value>(key: &str) -> Option<Value>
where
    Value: FromStr,
    <Value as FromStr>::Err: std::fmt::Debug
{
    env::var(key).map(|s| s.parse().expect("MCTSConfig: can't parse env variable")).ok()
}