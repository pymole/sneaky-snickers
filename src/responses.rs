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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Start {
    pub color: String,
    #[serde(rename = "headType")]
    pub head_type: HeadType,
    #[serde(rename = "tailType")]
    pub tail_type: TailType,
}

impl Start {
    pub fn new(color: String, head_type: HeadType, tail_type: TailType) -> Start {
        Start {
            color,
            head_type,
            tail_type,
        }
    }
}

// TODO: Make all the head types
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HeadType {
    Regular,
    Beluga,
    Bendr,
    Dead,
    Evil,
    Fang,
    Pixel,
    Safe,
    #[serde(rename = "sand-worm")]
    SandWorm,
    Shades,
    Smile,
    Tongue,
}

// TODO: Make all the tail types
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TailType {
    Regular,
    #[serde(rename = "block-bum")]
    BlockBum,
    Bolt,
    Curled,
    #[serde(rename = "fat-rattle")]
    FatRattle,
    Freckled,
    Hook,
    Pixel,
    #[serde(rename = "round-bum")]
    RoundBum,
    Sharp,
    Skinny,
    #[serde(rename = "small-rattle")]
    SmallRattle,
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

