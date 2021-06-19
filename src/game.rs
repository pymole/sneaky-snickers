use std::collections::VecDeque;

use crate::api;
use crate::vec2d::Vec2D;

pub struct Board {
    pub size: Point,
    pub food: Vec<Point>,
    pub snakes: Vec<Snake>,
    pub safe_zone: Rectangle,
}

pub struct Snake {
    pub health: i32,
    pub body: VecDeque<Point>,
}

pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Represents [p0.x, p1.x) × [p0.y, p1.y)
pub struct Rectangle {
    pub p0: Point,
    pub p1: Point,
}

impl Board {
    pub fn from_api(board_api: &api::objects::Board) -> Board {
        assert!(board_api.width > 0 && board_api.height > 0);

        Board {
            size: Point {
                x: board_api.width,
                y: board_api.height,
            },
            food: board_api.food.iter().map(Point::from_api).collect(),
            snakes: board_api.snakes.iter().map(Snake::from_api).collect(),
            safe_zone: Self::calcualate_safe_zone(&board_api),
        }
    }

    fn calcualate_safe_zone(board_api: &api::objects::Board) -> Rectangle {
        let mut has_hazard = Vec2D::init_same(board_api.width as usize, board_api.height as usize, false);

        for api::objects::Point{x, y} in board_api.hazards.iter() {
            has_hazard[(*x as usize, *y as usize)] = true;
        }

        let mut safe_zone = Rectangle {
            p0: Point { x: board_api.width, y: board_api.height },
            p1: Point { x: -1, y: -1 },
        };

        for x in 0..board_api.width {
            for y in 0..board_api.height {
                if !has_hazard[(x as usize, y as usize)] {
                    safe_zone.p0.x = safe_zone.p0.x.min(x);
                    safe_zone.p1.x = safe_zone.p1.x.max(x);
                    safe_zone.p0.y = safe_zone.p0.y.min(y);
                    safe_zone.p1.y = safe_zone.p1.y.max(y);
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
            body: snake_api.body.iter().map(Point::from_api).collect(),
        }
    }
}

impl Point {
    pub fn from_api(point_api: &api::objects::Point) -> Point {
        Point { x: point_api.x, y: point_api.y }
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

/*
    TODO:
    - directions
        - allow not to move
    - food placer
    - safe zone shrinker
    - maybe want to output events
        - food eaten
        - snake killed
*/

pub fn advance_one(board: &mut Board) {

}
