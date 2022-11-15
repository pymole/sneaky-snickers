use std::collections::VecDeque;
use std::fmt::{Debug, self};
use std::mem::{MaybeUninit, self};
use std::ops::{Add, AddAssign};
use std::hash::{Hash, Hasher};
use arrayvec::ArrayVec;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use serde::{Serialize, Deserialize};
use colored::*;

use crate::api;
use crate::api::objects::Movement;
use crate::vec2d::Vec2D;
use crate::zobrist::{ZobristHash, body_direction};

pub const MAX_SNAKE_COUNT: usize = 2;
pub const WIDTH: i32 = 11;
pub const HEIGHT: i32 = 11;
pub const SIZE: usize = (WIDTH * HEIGHT) as usize;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    // WARNING: Optimized for 2 players
    pub foods: Vec<Point>,
    pub snakes: [Snake; MAX_SNAKE_COUNT],
    pub turn: i32,
    pub hazard: Vec2D<bool>,
    pub hazard_start: Point,
    pub objects: Objects,
    pub zobrist_hash: ZobristHash,
    pub is_terminal: bool,
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
        let board_api = &state_api.board;
        debug_assert!(board_api.width == WIDTH && board_api.height == HEIGHT);

        Board::new(
            state_api.turn as i32,
            Some(state_api.board.food.clone()),
            None,
            Some(&state_api.board.hazards),
            Self::snake_api_to_snake_game(&state_api.board.snakes),
        )
    }

    pub fn new(
        turn: i32,
        foods: Option<Vec<Point>>,
        hazard_start: Option<Point>,
        hazards: Option<&Vec<Point>>,
        snakes: [Snake; MAX_SNAKE_COUNT],
    ) -> Board {
        debug_assert_eq!(MAX_SNAKE_COUNT, 2);

        // There always must be food.
        let mut foods = if let Some(foods) = foods {
            foods
        } else {
            Vec::new()
        };
        if foods.is_empty() {
            loop {
                let food = random_point_inside_borders();
                if snakes.iter().all(|snake| !snake.body.contains(&food)) {
                    foods.push(food);
                    break;
                }
            }
        }

        debug_assert!(snakes.iter().all(|snake| foods.iter().all(|food| !snake.body.contains(&food))));

        let zobrist_hash = Self::initial_zobrist_hash(
            turn,
            &snakes,
            &foods,
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
        self.objects.set_food_on_empty(pos);
        self.foods.push(pos);
        self.zobrist_hash.xor_food(pos);
    }

    fn calculate_objects(snakes: &[Snake; MAX_SNAKE_COUNT], foods: &Vec<Point>) -> Objects {
        let mut objects = Objects::new();

        for snake in snakes {
            for body_part in snake.body.iter().copied() {
                if !objects.is_body(body_part) {
                    objects.init_body(body_part);
                }
            }
        }

        for food in foods.iter().copied() {
            objects.init_food(food);
        }

        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let pos = Point {x, y};
                if objects.is_default(pos) {
                    objects.init_empty(pos);
                }
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
        foods: &Vec<Point>,
    ) -> ZobristHash {
        let mut zobrist_hash = ZobristHash::new();

        zobrist_hash.xor_turn(turn);
        for (snake_index, snake) in snakes.iter().enumerate() {
            for i in 1..snake.body.len() {
                let prev = snake.body[i - 1];
                let cur = snake.body[i];
                let direction = body_direction(cur, prev);
                zobrist_hash.xor_body_direction(cur, snake_index, direction);
                // TODO: Prevent unxor of Still on first turn
                // if direction == BodyDirections::Still {
                //     break
                // }
            }
        }

        for food in foods.iter().copied() {
            zobrist_hash.xor_food(food);
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

impl Hash for Board {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write_u64(self.zobrist_hash.get_value());
    }
}


pub type Object = u8;
pub const FOOD: u8 = u8::MAX;
pub const BODY: u8 = u8::MAX - 1;
pub const DEFAULT: u8 = u8::MAX - 2;
// EMPTY is dynamic and indexes to Objects.empties from 0 to 120.

pub fn is_empty(obj: Object) -> bool {
    obj < DEFAULT
}

#[derive(PartialEq, Clone, Eq, Debug, Serialize, Deserialize)]
pub struct Objects {
    pub map: [[Object; HEIGHT as usize]; WIDTH as usize],
    pub empties: ArrayVec<Point, SIZE>,
    // TODO: foods from 121 to 241
}

impl Objects {
    fn new() -> Objects {
        Objects {
            map: [[DEFAULT; HEIGHT as usize]; WIDTH as usize],
            empties: ArrayVec::new(),
        }
    }

    pub fn get(&self, pos: Point) -> Object {
        self.map[pos.x as usize][pos.y as usize]
    }

    pub fn empties_count(&self) -> usize {
        self.empties.len()
    }

    // Food

    pub fn set_food_on_empty(&mut self, pos: Point) {
        self._remove_empty(pos);
        self.map[pos.x as usize][pos.y as usize] = FOOD;
    }

    pub fn set_food_on_body(&mut self, pos: Point) {
        debug_assert!(self.is_body(pos));
        self.map[pos.x as usize][pos.y as usize] = FOOD;
    }

    pub fn init_food(&mut self, pos: Point) {
        debug_assert!(self.is_default(pos));
        self.map[pos.x as usize][pos.y as usize] = FOOD;
    }

    pub fn is_food(&self, pos: Point) -> bool {
        self.get(pos) == FOOD
    }

    // Body

    pub fn set_body_on_empty(&mut self, pos: Point) {
        self._remove_empty(pos);
        self.map[pos.x as usize][pos.y as usize] = BODY;
    }

    pub fn set_body_on_food(&mut self, pos: Point) {
        debug_assert!(self.is_food(pos));
        self.map[pos.x as usize][pos.y as usize] = BODY;
    }

    pub fn init_body(&mut self, pos: Point) {
        debug_assert!(self.is_default(pos));
        self.map[pos.x as usize][pos.y as usize] = BODY;
    }

    pub fn is_body(&self, pos: Point) -> bool {
        self.get(pos) == BODY
    }

    // Empty

    pub fn set_empty_on_body(&mut self, pos: Point) {
        debug_assert!(self.is_body(pos), "At {:?} there is {} not BODY", pos, self.get(pos));
        self._set_empty(pos);
    }

    pub fn set_empty_on_food(&mut self, pos: Point) {
        debug_assert!(self.is_food(pos));
        self._set_empty(pos);
    }

    pub fn init_empty(&mut self, pos: Point) {
        debug_assert!(self.is_default(pos));
        self._set_empty(pos);
    }

    pub fn is_empty(&self, pos: Point) -> bool {
        is_empty(self.get(pos))
    }

    fn _set_empty(&mut self, pos: Point) {
        let empty = self.empties.len() as u8;
        self.empties.push(pos);
        self.map[pos.x as usize][pos.y as usize] = empty;
    }

    fn _remove_empty(&mut self, pos: Point) {
        debug_assert!(self.is_empty(pos), "{:?} is not empty, it's {}", pos, self.get(pos));
        let i = self.map[pos.x as usize][pos.y as usize] as usize;
        self.empties.swap_remove(i);
        
        if i != self.empties.len() {
            // Now when where is new empty on this position
            // we must change map pointer
            let last_empty_pos = self.empties[i];
            self.map[last_empty_pos.x as usize][last_empty_pos.y as usize] = i as u8;
        }
    }

    pub fn get_empty_position(&self, rng: &mut impl rand::Rng) -> Option<Point> {
        self.empties.choose(rng).copied()
    }

    // Default

    pub fn is_default(&self, pos: Point) -> bool {
        self.map[pos.x as usize][pos.y as usize] == DEFAULT
    }

}

pub fn random_point_inside_borders() -> Point {
    let mut rnd = thread_rng();
    Point {
        x: rnd.gen_range(0..WIDTH),
        y: rnd.gen_range(0..HEIGHT),
    }
}

const SNAKE_COLORS: [(u8, u8, u8); MAX_SNAKE_COUNT] = [
    (252, 90, 50),
    (50, 90, 252),
];
const HAZARD_COLOR: (u8, u8, u8) = (64, 64, 64);
const FOOD_CHAR: &str = "*";
const DEAD_SNAKE_CHAR: &str = "X";

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut view: [[ColoredString; HEIGHT as usize]; WIDTH as usize] = Default::default();
        let mut output = String::new();

        for (i, snake) in self.snakes.iter().enumerate() {
            let color = SNAKE_COLORS[i];

            let health_count = (WIDTH as f32 * snake.health as f32 / 100.0) as usize;
            output += &"-".repeat(health_count).truecolor(color.0, color.1, color.2);
            output += " \n";

            for j in 1..snake.body.len() {
                let back = snake.body[j];
                let front = snake.body[j - 1];
                if back == front {
                    break
                }
                let d = body_direction(back, front);
                let fill = Movement::from_usize(d as usize).symbol().to_string().bold().truecolor(color.0, color.1, color.2);
                view[back.x as usize][back.y as usize] = fill;
            }
            
            let head = snake.head();
            let fill = if snake.is_alive() {
                i.to_string().truecolor(color.0, color.1, color.2)
            } else {
                DEAD_SNAKE_CHAR.truecolor(color.0, color.1, color.2)
            };
            
            view[head.x as usize][head.y as usize] = fill;
        }

        for food in &self.foods {
            view[food.x as usize][food.y as usize] = FOOD_CHAR.bright_green();
        }

        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let mut fill = view[x as usize][y as usize].clone();
                if fill.is_empty() {
                    fill = ColoredString::from(".");
                }

                if self.hazard[(x, y)] {
                    fill = fill.on_truecolor(HAZARD_COLOR.0, HAZARD_COLOR.1, HAZARD_COLOR.2)
                };
                
                view[x as usize][y as usize] = fill;
            }
        }

        output += "\n";

        for y in (0..HEIGHT as usize).rev() {
            for x in 0..WIDTH as usize {
                output += view[x][y].to_string().as_str();
            }
            output += "\n";
        }
        
        write!(f, "\n{}", output)
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
