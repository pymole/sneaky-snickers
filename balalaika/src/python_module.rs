use pyo3::prelude::*;
use pyo3::types::PyDict;
use crate::api::objects::State;
use crate::mcts::config::Config;
use crate::mcts::seq::{SequentialMCTS, SequentialMCTSConfig};
use crate::mcts::search::Search;
use crate::{features, mcts};
use crate::game::Board;
use crate::game_log as gl;
use crate::engine;
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

#[pyfunction]
fn get_nnue_features(
    board: &PyDict,
) -> PyResult<[bool; features::TOTAL_FEATURES_SIZE]> {
    let board: Board = depythonize(board).unwrap();
    let nnue_features = features::get_nnue_features(&board);
    Ok(nnue_features)
}

#[pyfunction]
fn get_board_from_state(
    py: Python<'_>,
    state: &PyDict,
) -> PyResult<PyObject> {
    let state: State = depythonize(state).unwrap();
    let board = Board::from_api(&state);
    let board = pythonize(py, &board).unwrap();
    Ok(board)
}

#[pyfunction]
fn search(
    board: &PyDict,
    iterations_count: usize,
) -> PyResult<[usize; MAX_SNAKE_COUNT]> {
    let board: Board = depythonize(board).unwrap();
    let config = SequentialMCTSConfig::from_env();
    let mut mcts = SequentialMCTS::new(config);
    mcts.search(&board, iterations_count);

    let mut actions = [0; MAX_SNAKE_COUNT];
    for i in 0..MAX_SNAKE_COUNT {
        let a = mcts.get_final_movement(&board, i);
        actions[i] = a as usize;
    }
    Ok(actions)
}

#[pyfunction]
fn get_masks(
    board: &PyDict,
) -> PyResult<[[bool; 4]; MAX_SNAKE_COUNT]> {
    let board: Board = depythonize(board).unwrap();
    let masks = mcts::utils::get_masks(&board);
    Ok(masks)
}

#[pyfunction]
fn advance_one_step(
    py: Python<'_>,
    board: &PyDict,
    actions: [usize; MAX_SNAKE_COUNT]
) -> PyResult<PyObject> {
    let mut board: Board = depythonize(board).unwrap();
    engine::advance_one_step(&mut board, actions);
    let board = pythonize(py, &board).unwrap();
    Ok(board)
}

#[pymodule]
fn balalaika(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rewind, m)?)?;
    m.add_function(wrap_pyfunction!(get_positions, m)?)?;
    m.add_function(wrap_pyfunction!(get_position, m)?)?;
    m.add_function(wrap_pyfunction!(flood_fill, m)?)?;
    m.add_function(wrap_pyfunction!(draw_board, m)?)?;
    m.add_function(wrap_pyfunction!(get_nnue_features, m)?)?;
    m.add_function(wrap_pyfunction!(get_board_from_state, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(get_masks, m)?)?;
    m.add_function(wrap_pyfunction!(advance_one_step, m)?)?;
    Ok(())
}