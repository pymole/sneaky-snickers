pub mod objects {
    use std::fmt;
    use serde::{Serialize, Deserialize};
    use crate::game::{GridPoint, CoordType};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct State {
        pub game: Game,
        pub turn: u32,
        pub board: Board,
        pub you: Snake,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct Game {
        pub id: String,
        pub ruleset: Ruleset,
        pub timeout: i32,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct Ruleset {
        pub name: String,
        pub version: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct Board {
        pub height: CoordType,
        pub width: CoordType,
        pub food: Vec<GridPoint>,
        pub hazards: Vec<GridPoint>,
        pub snakes: Vec<Snake>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct Snake {
        pub id: String,
        pub name: String,
        pub health: i32,
        pub body: Vec<GridPoint>,
        pub latency: String,
        pub head: GridPoint,
        pub length: u32,
        #[serde(default)]
        pub shout: String,
        // Present only in Squad Mode games
        // pub squad: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
    #[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
    pub enum Movement {
        Up = 0,
        Right = 1,
        Down = 2,
        Left = 3,
    }

    impl Movement {
        pub fn from_usize(value: usize) -> Movement {
            match value {
                0 => Movement::Up,
                1 => Movement::Right,
                2 => Movement::Down,
                3 => Movement::Left,
                _ => panic!("Out of range movement value"),
            }
        }

        pub fn symbol(&self) -> char {
            match self {
                Movement::Up => '↑',
                Movement::Right => '→',
                Movement::Down => '↓',
                Movement::Left => '←',
            }
        }
    }

    impl fmt::Display for Movement {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.symbol())
        }
    }
}

pub mod responses {
    use serde::{Deserialize, Serialize};
    use super::objects::Movement;

    #[derive(Serialize, Debug)]
    pub struct Info {
        pub apiversion: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub author: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub head: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<String>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct Move {
        #[serde(rename = "move")]
        movement: Movement,
        #[serde(skip_serializing_if = "Option::is_none")]
        shout: Option<String>,
    }

    impl Move {
        pub fn new(movement: Movement) -> Move {
            Move { movement , shout: None }
        }
    }
}
