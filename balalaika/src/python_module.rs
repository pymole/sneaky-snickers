use pyo3::prelude::*;
use pyo3::types::PyDict;
use crate::features;
use crate::game_log::{rewind, GameLog};
use pythonize::depythonize;


#[pyfunction]
fn get_positions(py: Python<'_>, game_log: &PyDict) -> PyResult<Vec<features::Position>> {
    let game_log: GameLog = depythonize(game_log).unwrap();
    let s = format!("print(\"{:?}\")", game_log.get_foods());
    let s = Box::leak(s.into_boxed_str());
    py.run(&s, None, None).expect("bang");
    let boards = rewind(&game_log);
    let positions = boards
        .iter()
        .map(features::get_position)
        .collect();
    Ok(positions)
}

#[pymodule]
fn balalaika(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_positions, m)?)?;
    Ok(())
}