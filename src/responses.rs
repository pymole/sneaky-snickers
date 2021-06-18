use rocket::serde::{Deserialize, Serialize};
use crate::objects::Movement;

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

