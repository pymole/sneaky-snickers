pub mod objects {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

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
        pub height: i32,
        pub width: i32,
        pub food: Vec<Point>,
        pub hazards: Vec<Point>,
        pub snakes: Vec<Snake>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct Snake {
        pub id: String,
        pub name: String,
        pub health: i32,
        pub body: Vec<Point>,
        pub latency: String,
        pub head: Point,
        pub length: u32,
        #[serde(default)]
        pub shout: String,
        // Present only in Squad Mode games
        // pub squad: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
    #[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
    pub enum Movement {
        Right,
        Left,
        Up,
        Down,
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

    // See https://play.battlesnake.com/references/customizations/ for images
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum HeadType {
        Default,

        // Standard
        Beluga,
        Bendr,
        Dead,
        Evil,
        Fang,
        Pixel,
        Safe,
        SandWorm,
        Shades,
        Silly,
        Smile,
        Tongue,

        // Battlesnake Winter Classic 2019
        Bonhomme,
        Earmuffs,
        Rudolph,
        Scarf,
        Ski,
        Snowman,
        SnowWorm,

        // Stay Home And Code 2020
        Caffeine,
        Gamer,
        TigerKing,
        Workout,
    }

    // See https://play.battlesnake.com/references/customizations/ for images
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    #[serde(rename_all = "kebab-case")]
    pub enum TailType {
        Default,

        // Standard
        BlockBum,
        Bolt,
        Curled,
        FatRattle,
        Freckled,
        Hook,
        Pixel,
        RoundBum,
        Sharp,
        Skinny,
        SmallRattle,

        // Battlesnake Winter Classic 2019
        Bonhomme,
        Flake,
        IceSkate,
        Present,

        // Stay Home And Code 2020
        Coffee,
        Mouse,
        TigerTail,
        Weight,
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
