use serde::{Serialize, Deserialize};

#[derive(Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct State {
    pub game: Game,
    pub turn: u32,
    pub board: Board,
    pub you: Snake,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Game {
    pub id: String,
    pub timeout: i32,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Board {
    pub height: i32,
    pub width: i32,
    pub food: Vec<Point>,
    pub snakes: Vec<Snake>,
    pub hazards: Vec<Point>,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Snake {
    pub id: String,
    pub name: String,
    pub health: i32,
    pub body: Vec<Point>,
    pub head: Point,
    pub length: u32,
    pub shout: String,
    pub latency: String,
}


#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Movement {
    Right,
    Left,
    Up,
    Down,
}
