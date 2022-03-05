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

use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;
use rocket::State;

use log::info;

use game::Board;
use engine::Movement;

use crate::mcts::config::MCTSConfig as _;
cfg_if::cfg_if! {
    if #[cfg(feature = "par")] {
        use crate::mcts::parallel::ParallelMCTS as MCTS;
        use crate::mcts::parallel::ParallelMCTSConfig as MCTSConfig;
    } else {
        use crate::mcts::seq::SequentialMCTS;
        use crate::mcts::seq::SequentialMCTSConfig as MCTSConfig;
    }
}

struct GameSession {
    mcts: Option<MCTS>,
    my_index: usize,
}

struct Storage {
    game_sessions: RwLock<HashMap<String, Mutex<GameSession>>>,
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
        let config = MCTSConfig::from_env();
        Some(MCTS::new(config))
    } else {
        None
    };

    let my_index = state.board.snakes
        .iter()
        .position(|snake| snake.id == state.you.id)
        .unwrap();

    let game_session = GameSession {
        mcts: mcts,
        my_index: my_index,
    };

    storage.game_sessions.write().unwrap().insert(state.game.id, Mutex::new(game_session));

    Status::Ok
}

// This route is needed for CORS
#[options("/move")]
fn movement_options() -> Status {
    Status::Ok
}

#[post("/move", data = "<body>")]
fn movement(storage: &State<Storage>, body: String) -> Json<api::responses::Move> {
    info!("MOVE - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();

    let game_sessions = storage.game_sessions.read().unwrap();

    let mut game_session = game_sessions.get(&state.game.id).unwrap().lock().unwrap();
    let my_index = game_session.my_index;
    let board = Board::from_api(&state);

    let movement = if let Some(mcts) = game_session.mcts.as_mut() {
        get_best_movement(mcts, &board, my_index)
    } else {
        let config = MCTSConfig::from_env();
        let mut mcts = MCTS::new(config);
        get_best_movement(&mut mcts, &board, my_index)
    };

    let move_ = api::responses::Move::new(movement);
    Json(move_)
}

#[post("/end", data = "<body>")]
fn end(storage: &State<Storage>, body: String) -> Status {
    info!("END - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    let mut game_session_mutex = storage.game_sessions.write().unwrap().remove(&state.game.id).unwrap();
    let game_session = game_session_mutex.get_mut().unwrap();
    let mcts = game_session.mcts.as_ref().unwrap();
    mcts.shutdown();

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
        .manage(Storage {game_sessions: RwLock::new(HashMap::new())})
        .mount("/", routes![index, start, movement, movement_options, end])
}
