use std::collections::VecDeque;
use std::ops::{Add, AddAssign};
use std::hash::{Hash, Hasher};
use crate::api;
use crate::vec2d::Vec2D;

pub const MAX_SNAKE_COUNT: usize = 8;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Board {
    pub size: Point,
    pub foods: Vec<Point>,
    pub snakes: Vec<Snake>,
    pub turn: i32,
    pub safe_zone: Rectangle,
    pub objects: Vec2D<Object>,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum Object {
    Empty,
    Food,
    BodyPart,
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct Snake {
    pub id: String,
    pub health: i32,
    pub body: VecDeque<Point>,
}

pub use crate::api::objects::Point;

/// Represents [p0.x, p1.x) × [p0.y, p1.y)
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub struct Rectangle {
    pub p0: Point,
    pub p1: Point,
}

impl Board {
    pub fn from_api(state_api: &api::objects::State) -> Board {
        let board_api = &state_api.board;
        assert!(board_api.width > 0 && board_api.height > 0);
        // TODO: validate that everything is inbounds.
        let objects = Self::calculate_objects(state_api);

        Board {
            size: Point {
                x: board_api.width,
                y: board_api.height,
            },
            foods: board_api.food.clone(), // TODO: reserve capacity
            snakes: board_api.snakes.iter().map(Snake::from_api).collect(),
            turn: state_api.turn as i32,
            safe_zone: Self::calcualate_safe_zone(&board_api.hazards, board_api.width, board_api.height),
            objects: objects,
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        Rectangle { p0: Point::ZERO, p1: self.size }.contains(p)
    }

    fn calculate_objects(state_api: &api::objects::State) -> Vec2D<Object> {
        let board_api = &state_api.board;

        let mut objects = Vec2D::init_same(
            board_api.width as usize,
            board_api.height as usize,
            Object::Empty,
        );

        for snake in board_api.snakes.iter() {
            if snake.health > 0 {
                for body_part in snake.body.iter() {
                    match objects[*body_part] {
                        Object::Empty => objects[*body_part] = Object::BodyPart,
                        Object::BodyPart => {} // A snake can intersect with itself in the begining and after eating a food.
                        Object::Food => unreachable!(),
                    }
                }
            }
        }

        for food in board_api.food.iter() {
            match objects[*food] {
                Object::Empty => objects[*food] = Object::Food,
                Object::BodyPart { .. } => unreachable!("Can't have food and snake body in the same square."),
                Object::Food => unreachable!("Can't have two food pieces in the same square."),
            }
        }

        objects
    }

    fn calcualate_safe_zone(hazards: &Vec<Point>, width: i32, height: i32) -> Rectangle {
        let mut safe_zone = Rectangle {
            p0: Point { x: width, y: height },
            p1: Point { x: -1, y: -1 },
        };

        for &Point{x, y} in hazards {
            safe_zone.p0.x = safe_zone.p0.x.min(x);
            safe_zone.p1.x = safe_zone.p1.x.max(x);
            safe_zone.p0.y = safe_zone.p0.y.min(y);
            safe_zone.p1.y = safe_zone.p1.y.max(y);
        }

        // Add one to include borders ( rectangle represents [p0.x, p1.x) × [p0.y, p1.y) )
        safe_zone.p1.x += 1;
        safe_zone.p1.y += 1;

        if safe_zone.empty() {
            Rectangle { p0: Point::ZERO, p1: Point::ZERO }
        }
        else {
            safe_zone
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.snakes.iter().filter(|snake| snake.is_alive()).count() < 2
    }
}

impl Hash for Board {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        (&self.snakes, &self.foods, &self.safe_zone).hash(state);
    }
}

impl Default for Object {
    fn default() -> Object {
        Object::Empty
    }
}

impl Snake {
    pub fn from_api(snake_api: &api::objects::Snake) -> Snake {
        assert!(snake_api.body.len() > 0);
        assert_eq!(snake_api.head, snake_api.body[0]);

        Snake {
            id: snake_api.id.clone(),
            health: snake_api.health,
            body: snake_api.body.iter().copied().collect(), // TODO: reserve some capacity
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

#[cfg(test)]
mod tests {
    #[test]
    fn board_from_api() {
        // TODO
    }
}

