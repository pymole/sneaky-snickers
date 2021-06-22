#![feature(proc_macro_hygiene, decl_macro)]

mod api;
mod game;
mod engine;
mod solver;
mod vec2d;

#[macro_use]
extern crate rocket;

use log::{info};

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;

use crate::solver::get_best_movement;


#[get("/")]
fn index() -> Json<api::responses::Info> {
    Json(api::responses::Info {
        apiversion: "1".to_string(),
        author: Some("Snickers".to_string()),
        color: Some("#b7410e".to_string()),
        head: None,
        tail: None,
        version: Some("0".to_string()),
    })
}

#[post("/start", data = "<body>")]
fn start(body: String) -> Status {
    info!("/start {}", body);
    Status::Ok
}

#[post("/move", data = "<body>")]
fn movement(body: String) -> Json<api::responses::Move> {
    info!("/move {}", body);

    let state = serde_json::from_str::<api::objects::State>(&body).unwrap();
    let movement = api::responses::Move::new(get_best_movement(state));

    Json(movement)
}

#[post("/end", data = "<body>")]
fn end(body: String) -> Status {
    info!("/end {}", body);
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    rocket::build().mount("/", routes![index, start, movement, end])
}
