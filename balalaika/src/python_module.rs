use pyo3::prelude::*;
use pyo3::types::PyDict;
use crate::{features, mcts};
use crate::game::Board;
use crate::game_log as gl;
use crate::game::{MAX_SNAKE_COUNT};
use pythonize::{depythonize, pythonize};
use mcts::heuristics;


#[pyfunction]
fn rewind(
    py: Python<'_>,
    game_log: &PyDict
) -> PyResult<(Vec<[usize; MAX_SNAKE_COUNT]>, PyObject, features::Rewards)> {
    let game_log: gl::GameLog = depythonize(game_log).unwrap();
    let (actions, boards) = gl::rewind(&game_log);
    let rewards = features::get_rewards(&boards[game_log.turns]);
    let boards = pythonize(py, &boards).unwrap();
    Ok((actions, boards, rewards))
}

#[pyfunction]
fn get_positions(
    _py: Python<'_>,
    game_log: &PyDict
) -> PyResult<(Vec<[usize; MAX_SNAKE_COUNT]>, Vec<features::Position>, features::Rewards)> {
    let game_log: gl::GameLog = depythonize(game_log).unwrap();
    let (actions, boards) = gl::rewind(&game_log);
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

#[pyfunction]
fn get_position(
    _py: Python<'_>,
    board: &PyDict
) -> PyResult<features::Position> {
    let board: Board = depythonize(board).unwrap();
    let position = features::get_position(&board);
    Ok(position)
}

#[pyfunction]
fn flood_fill(
    board: &PyDict,
) -> heuristics::flood_fill::FloodFill {
    let board: Board = depythonize(board).unwrap();
    heuristics::flood_fill::flood_fill(&board)
}

#[pyfunction]
fn draw_board(
    board: &PyDict,
) {
    let board: Board = depythonize(board).unwrap();
    println!("{}", board);
}

#[pymodule]
fn balalaika(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rewind, m)?)?;
    m.add_function(wrap_pyfunction!(get_positions, m)?)?;
    m.add_function(wrap_pyfunction!(get_position, m)?)?;
    m.add_function(wrap_pyfunction!(flood_fill, m)?)?;
    m.add_function(wrap_pyfunction!(draw_board, m)?)?;
    Ok(())
}