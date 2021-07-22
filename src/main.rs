#![feature(proc_macro_hygiene, decl_macro)]

mod api;
mod game;
mod engine;
mod solver;
mod vec2d;
mod mcts;

#[cfg(test)]
mod test_data;

#[macro_use]
extern crate rocket;

use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;

use log::{info};
use std::env;

use crate::game::Board;
use crate::mcts::MCTS;

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
fn start(body: String) -> Status {
    info!("START - {}", body);
    Status::Ok
}

// This route is needed for CORS
#[options("/move")]
fn movement_options() -> Status {
    Status::Ok
}

#[post("/move", data = "<body>")]
fn movement(body: String) -> Json<api::responses::Move> {
    info!("MOVE - {}", body);

    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    let board = Board::from_api(&state);

    let mut mcts = MCTS::new(2.0);

    // TODO: Config
    let iterations = env::var("MCTS_ITERATIONS").unwrap().parse().unwrap();
    mcts.search(&board, iterations);

    let my_index = state.board.snakes
        .iter()
        .position(|snake| snake.id == state.you.id)
        .unwrap();

    let movement = api::responses::Move::new(mcts.get_movement(&board, my_index));
    Json(movement)
}

#[post("/end", data = "<body>")]
fn end(body: String) -> Status {
    info!("END - {}", body);
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
        .mount("/", routes![index, start, movement, movement_options, end, flavored_flood_fill])
}
