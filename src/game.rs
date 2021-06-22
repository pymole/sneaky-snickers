use std::collections::VecDeque;
use std::ops::{Add, AddAssign};

use crate::api;
use crate::vec2d::Vec2D;

pub const MAX_SNAKE_COUNT : usize = 8;

pub struct Board {
    pub size: Point,
    pub foods: Vec<Point>,
    pub snakes: Vec<Snake>,
    pub turn: i32,
    pub safe_zone: Rectangle,
    pub squares: Vec2D<Square>,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Object {
    Empty,
    Food,
    BodyPart,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Square {
    pub safe: bool,
    pub object: Object,
}

pub struct Snake {
    pub health: i32,
    pub body: VecDeque<Point>,
}

pub use crate::api::objects::Point;

/// Represents [p0.x, p1.x) × [p0.y, p1.y)
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Rectangle {
    pub p0: Point,
    pub p1: Point,
}

impl Board {
    pub fn from_api(state_api: &api::objects::State) -> Board {
        let board_api = &state_api.board;
        assert!(board_api.width > 0 && board_api.height > 0);
        // TODO: validate that everything is inbounds.
        let squares = Self::calculate_squares(state_api);

        Board {
            size: Point {
                x: board_api.width,
                y: board_api.height,
            },
            foods: board_api.food.clone(),
            snakes: board_api.snakes.iter().map(Snake::from_api).collect(),
            turn: state_api.turn as i32,
            safe_zone: Self::calcualate_safe_zone(&squares),
            squares: squares,
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        Rectangle { p0: Point::ZERO, p1: self.size }.contains(p)
    }

    fn calculate_squares(state_api: &api::objects::State) -> Vec2D<Square> {
        let board_api = &state_api.board;

        let mut squares = Vec2D::init_same(
            board_api.width as usize,
            board_api.height as usize,
            Square {
                safe: true,
                object: Object::Empty,
            }
        );

        for p in board_api.hazards.iter() {
            squares[*p].safe = false;
        }

        for snake in board_api.snakes.iter() {
            for (i, body_part) in snake.body.iter().enumerate() {
                match squares[*body_part].object {
                    Object::Empty => squares[*body_part].object = Object::BodyPart,
                    Object::BodyPart => {} // A snake can intersect with itself in the begining and after eating a food.
                    Object::Food => unreachable!(),
                }
            }
        }

        for food in board_api.food.iter() {
            match squares[*food].object {
                Object::Empty => squares[*food].object = Object::Food,
                Object::BodyPart { .. } => unreachable!("Can't have food and snake body in the same square."),
                Object::Food => unreachable!("Can't have two food pieces in the same square."),
            }
        }

        squares
    }

    fn calcualate_safe_zone(squares: &Vec2D<Square>) -> Rectangle {
        let mut safe_zone = Rectangle {
            p0: Point { x: squares.len1 as i32, y: squares.len2 as i32 },
            p1: Point { x: -1, y: -1 },
        };

        for x in 0..squares.len1 {
            for y in 0..squares.len2 {
                if !squares[(x, y)].safe {
                    safe_zone.p0.x = safe_zone.p0.x.min(x as i32);
                    safe_zone.p1.x = safe_zone.p1.x.max(x as i32);
                    safe_zone.p0.y = safe_zone.p0.y.min(y as i32);
                    safe_zone.p1.y = safe_zone.p1.y.max(y as i32);
                }
            }
        }

        if safe_zone.empty() {
            Rectangle { p0: Point { x: 0, y: 0 }, p1: Point { x: 0, y: 0 } }
        }
        else {
            safe_zone
        }
    }
}

impl Snake {
    pub fn from_api(snake_api: &api::objects::Snake) -> Snake {
        assert!(snake_api.body.len() > 0);
        assert_eq!(snake_api.head, snake_api.body[0]);

        Snake {
            health: snake_api.health,
            body: snake_api.body.iter().copied().collect(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

impl Point {
    pub const ZERO: Point = Point { x: 0, y: 0 };
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Point) {
        *self = *self + other;
    }
}

impl Rectangle {
    pub fn contains(&self, p: Point) -> bool {
        self.p0.x <= p.x && p.x < self.p1.x &&
        self.p0.y <= p.y && p.y < self.p1.y
    }

    pub fn empty(&self) -> bool {
        self.p0.x >= self.p1.x ||
        self.p0.y >= self.p1.y
    }
}
