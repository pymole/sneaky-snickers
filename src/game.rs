use std::collections::VecDeque;
use std::mem::{MaybeUninit, self};
use std::ops::{Add, AddAssign};
use std::hash::{Hash, Hasher};
use rand::thread_rng;
use rand::Rng;
use serde::{Serialize, Deserialize};

use crate::api;
use crate::vec2d::Vec2D;
use crate::zobrist::{ZobristHash, body_direction};

pub const MAX_SNAKE_COUNT: usize = 2;
pub const WIDTH: i32 = 11;
pub const HEIGHT: i32 = 11;
pub const SIZE: usize = (WIDTH * HEIGHT) as usize;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Board {
    // WARNING: Optimized for 2 player
    pub foods: Vec<Point>,
    pub snakes: [Snake; MAX_SNAKE_COUNT],
    pub turn: i32,
    pub hazard: Vec2D<bool>,
    pub hazard_start: Point,
    pub objects: Vec2D<Object>,
    pub zobrist_hash: ZobristHash,
    pub is_terminal: bool,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum Object {
    BodyPart,
    Food,
    Empty,
}

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct Snake {
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
        debug_assert!(state_api.board.snakes.len() == 2);
        
        let board_api = &state_api.board;
        debug_assert!(board_api.width == WIDTH && board_api.height == HEIGHT);

        Board::new(
            state_api.turn as i32,
            state_api.board.food.clone(),
            None,
            Some(&state_api.board.hazards),
            Self::snake_api_to_snake_game(&state_api.board.snakes),
        )
    }

    pub fn new(
        turn: i32,
        foods: Vec<Point>,
        hazard_start: Option<Point>,
        hazards: Option<&Vec<Point>>,
        snakes: [Snake; MAX_SNAKE_COUNT],
    ) -> Board {
        debug_assert!(MAX_SNAKE_COUNT == 2);

        let zobrist_hash = Self::initial_zobrist_hash(
            turn,
            &snakes,
        );

        let hazard_start_;
        let hazards_;

        let empty_hazards = Vec::new();

        if hazard_start.is_some() {
            assert!(hazards.is_none());
            hazards_ = &empty_hazards;
            hazard_start_ = hazard_start.unwrap();
        } else if hazards.is_some() && hazards.unwrap().len() != 0 {
            let hazards = hazards.unwrap();
            hazards_ = hazards;
            hazard_start_ = hazards[0];
        } else {
            hazards_ = &empty_hazards;
            hazard_start_ = random_point_inside_borders();
        };

        let objects = Self::calculate_objects(&snakes, &foods);

        Board {
            foods,
            snakes,
            turn,
            hazard: Self::calcualate_hazard(hazards_),
            hazard_start: hazard_start_,
            objects,
            zobrist_hash,
            is_terminal: false,
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        (p.x < WIDTH) && (p.y < HEIGHT) && (p.x >= 0) && (p.y >= 0)
    }

    pub fn is_terminal(&self) -> bool {
        self.is_terminal
    }

    pub fn put_food(&mut self, pos: Point) {
        self.objects[pos] = Object::Food;
        self.zobrist_hash.xor_food(pos);
        self.foods.push(pos);
    }

    fn calculate_objects(snakes: &[Snake; MAX_SNAKE_COUNT], foods: &Vec<Point>) -> Vec2D<Object> {
        let mut objects = Vec2D::init_same(
            WIDTH as usize,
            HEIGHT as usize,
            Object::Empty,
        );

        for snake in snakes {
            for body_part in snake.body.iter().copied() {
                match objects[body_part] {
                    Object::Empty => objects[body_part] = Object::BodyPart,
                    Object::BodyPart => {} // A snake can intersect with itself in the begining and after eating a food.
                    _ => unreachable!(),
                }
            }
        }

        for food in foods.iter().copied() {
            match objects[food] {
                Object::Empty => objects[food] = Object::Food,
                Object::BodyPart { .. } => unreachable!("Can't have food and snake body in the same square."),
                Object::Food => unreachable!("Can't have two food pieces in the same square."),
            }
        }

        objects
    }

    fn calcualate_hazard(hazards : &Vec<Point>) -> Vec2D<bool> {
        let mut hazard = Vec2D::init_same(WIDTH as usize, HEIGHT as usize, false);

        for &p in hazards {
            hazard[p] = true;
        }

        hazard
    }

    fn initial_zobrist_hash(
        turn: i32,
        snakes: &[Snake; MAX_SNAKE_COUNT],
    ) -> ZobristHash {
        let mut zobrist_hash = ZobristHash::new();

        zobrist_hash.xor_turn(turn);
        for (snake_index, snake) in snakes.iter().enumerate() {
            for i in 1..snake.body.len() {
                let prev = snake.body[i - 1];
                let cur = snake.body[i];
                let direction = body_direction(cur, prev);
                zobrist_hash.xor_body_direction(cur, snake_index, direction);
            }
        }

        zobrist_hash
    }

    pub fn snake_api_to_snake_game(snakes_api: &Vec<api::objects::Snake>) -> [Snake; MAX_SNAKE_COUNT] {
        let mut snakes: [MaybeUninit<Snake>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, snake) in snakes_api.iter().enumerate() {
            snakes[i] = MaybeUninit::new(Snake::from_api(snake));
        }

        unsafe { mem::transmute(snakes) }
    }
}

pub fn random_point_inside_borders() -> Point {
    let mut rnd = thread_rng();
    Point {
        x: rnd.gen_range(0..WIDTH),
        y: rnd.gen_range(0..HEIGHT),
    }
}

impl Hash for Board {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write_u64(self.zobrist_hash.get_value());
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
            health: snake_api.health,
            body: snake_api.body.iter().copied().collect(), // TODO: reserve some capacity
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn head(&self) -> Point {
        self.body[0]
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

    #[allow(dead_code)]
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
