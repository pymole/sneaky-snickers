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
        author: Some("Snickers".to_string()),
        color: Some("#b7410e".to_string()),
        head: None,
        tail: None,
        version: Some("0".to_string()),
    })
}

#[post("/start", data = "<_state>")]
fn start(_state: Json<objects::State>) -> Status {
    Status::Ok
}

#[post("/move", data = "<state>")]
fn movement(state: Json<objects::State>) -> Json<responses::Move> {
    let movement = Move::new(get_best_movement((*state).clone()));
    Json(movement)
}

#[post("/end", data = "<_state>")]
fn end(_state: Json<objects::State>) -> Status {
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, start, movement, end])
}
