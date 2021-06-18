#![feature(proc_macro_hygiene, decl_macro)]

mod objects;
mod responses;
mod solver;

#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::serde::json::Json;

use crate::solver::get_best_movement;
use crate::responses::Move;


#[get("/")]
fn index() -> Json<responses::Info> {
    Json(responses::Info {
        apiversion: "1".to_string(),
        author: None,
        color: Some("#b7410e".to_string()),
        head: None,
        tail: None,
        version: Some("0".to_string()),
    })
}

#[post("/start")]
fn start() -> Status {
    Status::Ok
}

#[post("/move", data = "<state>")]
fn movement(state: Json<objects::State>) -> Json<responses::Move> {
    let movement = Move::new(get_best_movement((*state).clone()));
    Json(movement)
}

#[post("/end")]
fn end() -> Status {
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, start, movement, end])
}
