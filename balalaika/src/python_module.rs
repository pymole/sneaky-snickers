use pyo3::prelude::*;
use pyo3::types::PyDict;
use crate::api::objects::State;
use crate::mcts::config::Config;
use crate::mcts::seq::{SequentialMCTS, SequentialMCTSConfig};
use crate::mcts::search::Search;
use crate::{features, mcts, dataloader};
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
fn get_examples_from_board(
    board: &PyDict,
    rewards: [f32; MAX_SNAKE_COUNT],
) -> PyResult<Vec<features::Example>> {
    let board: Board = depythonize(board).unwrap();
    let examples = features::collect_examples(&board, rewards);
    Ok(examples)
}

#[pyfunction]
fn get_examples_from_game_log(
    game_log: &PyDict,
) -> PyResult<Vec<features::Example>> {
    let game_log: gl::GameLog = depythonize(game_log).unwrap();
    let (_, boards) = gl::rewind(&game_log);
    let (terminal_board, boards) = boards.split_last().unwrap();
    let (rewards, _) = features::get_rewards(terminal_board);
    let mut examples = Vec::new();
    // Do not use terminal board in examples
    for board in boards {
        examples.extend(features::collect_examples(&board, rewards));
    }
    
    Ok(examples)
}

#[pyfunction]
fn get_features(
    board: &PyDict,
) -> PyResult<Vec<usize>> {
    let board: Board = depythonize(board).unwrap();
    let indices = features::get_features_indices(&board);
    Ok(indices)
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

#[pyclass]
pub struct DataLoader {
    provider: dataloader::DataLoader,
}

#[pymethods]
impl DataLoader {
    #[new]
    pub fn new(
        mongo_uri: String,
        batch_size: usize,
        prefetch_batches: usize,
        mixer_size: usize,
        game_log_ids: Vec<String>,
    ) -> Self {
        let dataloader = dataloader::DataLoader::new(
            mongo_uri,
            batch_size,
            prefetch_batches,
            mixer_size,
            game_log_ids,
        );

        Self {
            provider: dataloader,
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Vec<features::Example>> {
        slf.provider.next()
    }
}

#[pymodule]
fn balalaika(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rewind, m)?)?;
    m.add_function(wrap_pyfunction!(flood_fill, m)?)?;
    m.add_function(wrap_pyfunction!(draw_board, m)?)?;
    m.add_function(wrap_pyfunction!(get_examples_from_board, m)?)?;
    m.add_function(wrap_pyfunction!(get_examples_from_game_log, m)?)?;
    m.add_function(wrap_pyfunction!(get_board_from_state, m)?)?;
    m.add_function(wrap_pyfunction!(get_features, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(get_masks, m)?)?;
    m.add_function(wrap_pyfunction!(advance_one_step, m)?)?;
    m.add_class::<DataLoader>()?;
    Ok(())
}