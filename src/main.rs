// #![feature(proc_macro_hygiene, decl_macro, total_cmp)]
mod api;
mod game;
mod engine;
mod bandit;
mod ucb;
mod solver;
mod vec2d;
mod mcts;
mod parallelization;

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
use mcts::{MCTSConfig, MCTS, GetBestMovement};
// use parallelization::root_parallelization;
use engine::Movement;


struct Storage {
    mcts_instances: RwLock<HashMap<String, Mutex<MCTS>>>,
}


fn get_best_movement(mcts: &mut MCTS, board: &Board, agent: usize) -> Movement {
    if let Some(search_time) = mcts.config.search_time {
        mcts.search_with_time(&board, search_time);
    } else if let Some(iterations) = mcts.config.iterations {
        mcts.search(&board, iterations);
    } else {
        panic!("Provide MCTS_SEARCH_TIME or MCTS_ITERATIONS");
    }

    mcts.get_best_movement(&board, agent)
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
    if env::var("MCTS_PERSISTENT").is_ok() {
        let state = serde_json::from_str::<api::objects::State>(&body).unwrap();

        let config = MCTSConfig::from_env();
        let mcts = MCTS::new(config);

        storage.mcts_instances.write().unwrap().insert(state.game.id, Mutex::new(mcts));
    }

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

    let board = Board::from_api(&state);

    let my_index = state.board.snakes
        .iter()
        .position(|snake| snake.id == state.you.id)
        .unwrap();

    let movement = if let Some(mcts_mutex) = storage.mcts_instances.read().unwrap().get(&state.game.id) {
        let mut mcts = mcts_mutex.lock().unwrap();
        get_best_movement(&mut mcts, &board, my_index)
    } else {
        let config = MCTSConfig::from_env();
        let mcts = &mut MCTS::new(config);
        get_best_movement(mcts, &board, my_index)
    };

    let movement = api::responses::Move::new(movement);
    Json(movement)
}

#[post("/end", data = "<body>")]
fn end(storage: &State<Storage>, body: String) -> Status {
    info!("END - {}", body);
    if env::var("MCTS_PERSISTENT").is_ok() {
        let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
        storage.mcts_instances.write().unwrap().remove(&state.game.id);
    }
    Status::Ok
}

#[post("/flavored_flood_fill", data = "<body>")]
fn flavored_flood_fill(body: String) -> Json<solver::FloodFill> {
    info!("FLOOD - {}", body);
    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    let board = Board::from_api(&state);

    Json(solver::flavored_flood_fill(&board))
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
        .manage(Storage {mcts_instances: RwLock::new(HashMap::new())})
        .mount("/", routes![index, start, movement, movement_options, end, flavored_flood_fill])
}
