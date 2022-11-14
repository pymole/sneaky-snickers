use pyo3::prelude::*;
use pyo3::types::PyDict;
use crate::features;
use crate::game_log::{rewind, GameLog};
use crate::game::MAX_SNAKE_COUNT;
use pythonize::depythonize;

#[pyfunction]
fn get_positions(
    _py: Python<'_>,
    game_log: &PyDict
) -> PyResult<(Vec<[usize; MAX_SNAKE_COUNT]>, Vec<features::Position>, ([f32; MAX_SNAKE_COUNT], bool))> {
    let game_log: GameLog = depythonize(game_log).unwrap();
    let (actions, boards) = rewind(&game_log);
    // let s = format!("print(\"{:?}{}\")", &boards[game_log.turns], &boards[game_log.turns]);
    // let s = Box::leak(s.into_boxed_str());
    // py.run(&s, None, None).expect("bang");
    let positions = boards
        .iter()
        .map(features::get_position)
        .collect();
    let rewards = features::get_rewards(&boards[game_log.turns]);
    Ok((actions, positions, rewards))
}

#[pymodule]
fn balalaika(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_positions, m)?)?;
    Ok(())
}