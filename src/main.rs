// #![feature(proc_macro_hygiene, decl_macro, total_cmp)]
mod api;
mod game;
mod engine;
mod vec2d;
mod mcts;
mod zobrist;

#[cfg(test)]
mod test_data;

#[macro_use]
extern crate rocket;

use std::env;

use std::sync::Mutex;

use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::http::Status;
use rocket::routes;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;
use rocket::State;
use dashmap::DashMap;

use log::info;

use game::Board;
use engine::Movement;

use crate::mcts::config::MCTSConfig as _;
cfg_if::cfg_if! {
    if #[cfg(feature = "par")] {
        use crate::mcts::parallel::ParallelMCTS as MCTS;
        use crate::mcts::parallel::ParallelMCTSConfig as MCTSConfig;
    } else {
        use crate::mcts::seq::SequentialMCTS as MCTS;
        use crate::mcts::seq::SequentialMCTSConfig as MCTSConfig;
    }
}

struct GameSession {
    mcts: Option<MCTS>,
}

struct Storage {
    game_sessions: DashMap<String, Mutex<GameSession>>,
}


fn get_best_movement(mcts: &mut MCTS, board: &Board, agent: usize) -> Movement {
    if let Some(search_time) = mcts.config.search_time {
        mcts.search_with_time(board, search_time);
    } else if let Some(iterations) = mcts.config.iterations {
        mcts.search(board, iterations);
    } else {
        panic!("Provide MCTS_SEARCH_TIME or MCTS_ITERATIONS");
    }

    mcts.get_final_movement(board, agent)
}

fn our_snake_index(state: &api::objects::State) -> usize {
    state.board.snakes
        .iter()
        .position(|snake| snake.id == state.you.id)
        .unwrap()
}

#[get("/")]
fn index() -> Json<api::responses::Info> {
    Json(api::responses::Info {
        apiversion: "1".to_string(),
        author: Some("Snickers".to_string()),
        color: Some("#4a3120".to_string()),
        head: Some("bendr".to_string()),
        tail: Some("round-bum".to_string()),
        version: Some("0".to_string()),
    })
}

#[post("/start", data = "<body>")]
fn start(body: String, storage: &State<Storage>) -> Status {
    info!("START - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();

    let mcts = if env::var("MCTS_PERSISTENT").is_ok() {
        Some(MCTS::new(MCTSConfig::from_env()))
    } else {
        None
    };

    let game_session = GameSession {
        mcts: mcts,
    };

    if let Some(mut game_session_mutex) = storage.game_sessions.insert(state.game.id, Mutex::new(game_session)) {
        warn!("Game with given id already exists! Replacing...");
        // TODO: copy-pasta
        let game_session = game_session_mutex.get_mut().unwrap();
        if let Some(mcts) = game_session.mcts.as_ref() {
            mcts.shutdown();
        }
    }

    Status::Ok
}

// This route is needed for CORS
#[options("/move")]
fn movement_options() -> Status {
    Status::Ok
}

fn try_move_using_existing_game_session(storage: &State<Storage>, state: &api::objects::State) -> Option<Movement> {
    if let Some(game_session_mutex) = storage.game_sessions.get(&state.game.id) {
        if let Some(mcts) = game_session_mutex.lock().unwrap().mcts.as_mut() {
            let board = Board::from_api(&state);
            return Some(get_best_movement(mcts, &board, our_snake_index(&state)));
        }
    }

    None
}

#[post("/move", data = "<body>")]
fn movement(storage: &State<Storage>, body: String) -> Json<api::responses::Move> {
    info!("MOVE - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();

    let movement = try_move_using_existing_game_session(storage, &state).unwrap_or_else(|| {
        let board = Board::from_api(&state);
        let mut mcts = MCTS::new(MCTSConfig::from_env());
        let movement = get_best_movement(&mut mcts, &board, our_snake_index(&state));
        mcts.shutdown();
        movement
    });

    Json(api::responses::Move::new(movement))
}

// This route is needed for CORS
#[options("/flood_fill")]
fn ff_options() -> Status {
    Status::Ok
}

#[post("/flood_fill", data = "<body>")]
fn flood_fill(body: String) -> Json<mcts::heuristics::flood_fill::FloodFill> {
    info!("FLOOD - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    let board = Board::from_api(&state);

    let f = mcts::heuristics::flood_fill::flood_fill_estimate(&board);
    info!("{:?}", f);

    Json(mcts::heuristics::flood_fill::flood_fill(&board))
}

#[post("/end", data = "<body>")]
fn end(storage: &State<Storage>, body: String) -> Status {
    info!("END - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    if let Some((_, mut game_session_mutex)) = storage.game_sessions.remove(&state.game.id) {
        let game_session = game_session_mutex.get_mut().unwrap();
        if let Some(mcts) = game_session.mcts.as_ref() {
            mcts.shutdown();
        }
    }

    Status::Ok
}

#[launch]
fn rocket() -> _ {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    rocket::build()
        .attach(AdHoc::on_response("Cors", |_, response| Box::pin(async move {
            response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
            response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
            response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
            response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        })))
        .manage(Storage {game_sessions: DashMap::new()})
        .mount("/", routes![index, start, movement, movement_options, end, flood_fill, ff_options])
}
