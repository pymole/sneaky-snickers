#![feature(proc_macro_hygiene, decl_macro)]

mod objects;
mod responses;
mod solver;

#[macro_use]
extern crate rocket;

use log::{info};

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;

use crate::solver::get_best_movement;
use crate::responses::Move;


#[get("/")]
fn index() -> Json<responses::Info> {
    Json(responses::Info {
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
fn movement(body: String) -> Json<responses::Move> {
    info!("/move {}", body);

    let state = serde_json::from_str::<objects::State>(&body).unwrap();
    let movement = Move::new(get_best_movement(state));

    Json(movement)
}

#[post("/end", data = "<body>")]
fn end(body: String) -> Status {
    info!("/end {}", body);
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    use env_logger;
    env_logger::init();

    rocket::build().mount("/", routes![index, start, movement, end])
}
