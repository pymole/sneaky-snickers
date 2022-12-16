use std::collections::VecDeque;
use std::fmt::{Debug, self};
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
use crate::array2d::Array2D;
use crate::zobrist::{ZobristHash, body_direction};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

pub type PointUsize = Point<usize>;

pub type CoordType = i32;
pub type GridPoint = Point<CoordType>;

pub const MAX_SNAKE_COUNT: usize = 4;
pub const WIDTH: CoordType = 11;
pub const HEIGHT: CoordType = 11;
pub const CENTER: GridPoint = Point {x: WIDTH / 2, y: HEIGHT / 2};
pub const SIZE: usize = (WIDTH * HEIGHT) as usize;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub foods: Vec<GridPoint>,
    pub snakes: ArrayVec<Snake, MAX_SNAKE_COUNT>,
    pub turn: i32,
    pub safe_zone: Rectangle,
    pub objects: Objects,
    pub zobrist_hash: ZobristHash,
    pub is_terminal: bool,
}

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct Snake {
    pub health: i32,
    pub body: VecDeque<GridPoint>,
}

/// Represents [p0.x, p1.x) × [p0.y, p1.y)
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct Rectangle {
    pub p0: GridPoint,
    pub p1: GridPoint,
}

impl Board {
    pub fn from_api(state_api: &api::objects::State) -> Board {        
        let board_api = &state_api.board;
        debug_assert!(board_api.width == WIDTH && board_api.height == HEIGHT);

        Board::new(
            state_api.turn as i32,
            Some(state_api.board.food.clone()),
            Some(Self::calculate_safe_zone(&state_api.board.hazards)),
            Self::snake_api_to_snake_game(&state_api.board.snakes),
        )
    }

    pub fn new(
        turn: i32,
        foods: Option<Vec<GridPoint>>,
        safe_zone: Option<Rectangle>,
        snakes: ArrayVec<Snake, MAX_SNAKE_COUNT>,
    ) -> Board {
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

        let safe_zone = if safe_zone.is_some() {
            safe_zone.unwrap()
        } else {
            Rectangle {
                p0: Point {x: 0, y: 0},
                p1: Point {x: WIDTH, y: HEIGHT},
            }
        };

        let objects = Self::calculate_objects(&snakes, &foods);

        Board {
            foods,
            snakes,
            turn,
            safe_zone,
            objects,
            zobrist_hash,
            is_terminal: false,
        }
    }

    pub fn contains(&self, p: GridPoint) -> bool {
        (p.x < WIDTH) && (p.y < HEIGHT) && (p.x >= 0) && (p.y >= 0)
    }

    pub fn is_hazard(&self, p: GridPoint) -> bool {
        !self.safe_zone.contains(p)
    }

    pub fn is_terminal(&self) -> bool {
        (0..self.snakes.len()).filter(|&i| self.snakes[i].is_alive()).count() <= 1
    }

    pub fn put_food(&mut self, pos: GridPoint) {
        self.objects.set_food_on_empty(pos.into());
        self.foods.push(pos);
        self.zobrist_hash.xor_food(pos.into());
    }

    fn calculate_objects(snakes: &ArrayVec<Snake, MAX_SNAKE_COUNT>, foods: &Vec<GridPoint>) -> Objects {
        let mut objects = Objects::new();

        for snake in snakes {
            for body_part in snake.body.iter().copied() {
                if !objects.is_body(body_part.into()) {
                    objects.init_body(body_part.into());
                }
            }
        }

        for food in foods.iter().copied() {
            objects.init_food(food.into());
        }

        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                let pos = Point {x, y};
                if objects.is_default(pos.into()) {
                    objects.init_empty(pos.into());
                }
            }
        }

        objects
    }

    pub fn calculate_safe_zone(hazards: &Vec<GridPoint>) -> Rectangle {
        let is_safe = {
            let mut mask = Array2D::init_same(true);

            for &p in hazards {
                let p: PointUsize = p.into();
                mask[(p.x, p.y)] = false;
            }

            mask
        };

        let mut safe_zone = Rectangle {
            p0: GridPoint { x: WIDTH, y: HEIGHT },
            p1: GridPoint { x: -1, y: -1 },
        };

        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                if is_safe[(x as usize, y as usize)] {
                    safe_zone.p0.x = safe_zone.p0.x.min(x);
                    safe_zone.p1.x = safe_zone.p1.x.max(x);
                    safe_zone.p0.y = safe_zone.p0.y.min(y);
                    safe_zone.p1.y = safe_zone.p1.y.max(y);
                }
            }
        }

        // Add one to include borders ( rectangle represents [p0.x, p1.x) × [p0.y, p1.y) )
        safe_zone.p1.x += 1;
        safe_zone.p1.y += 1;

        if safe_zone.empty() {
            Rectangle { p0: Point::ZERO, p1: Point::ZERO }
        } else {
            safe_zone
        }
    }

    fn initial_zobrist_hash(
        turn: i32,
        snakes: &ArrayVec<Snake, MAX_SNAKE_COUNT>,
        foods: &Vec<GridPoint>,
    ) -> ZobristHash {
        let mut zobrist_hash = ZobristHash::new();

        zobrist_hash.xor_turn(turn);
        for (snake_index, snake) in snakes.iter().enumerate() {
            for i in 1..snake.body.len() {
                let prev = snake.body[i - 1];
                let cur = snake.body[i];
                let direction = body_direction(cur, prev);
                zobrist_hash.xor_body_direction(cur.into(), snake_index, direction);
                // TODO: Prevent unxor of Still on first turn
                // if direction == BodyDirections::Still {
                //     break
                // }
            }
        }

        for food in foods.iter().copied() {
            zobrist_hash.xor_food(food.into());
        }

        zobrist_hash
    }

    pub fn snake_api_to_snake_game(snakes_api: &Vec<api::objects::Snake>) -> ArrayVec<Snake, MAX_SNAKE_COUNT> {
        snakes_api.iter().map(Snake::from_api).collect()
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
    pub empties: ArrayVec<PointUsize, SIZE>,
    // TODO: foods from 121 to 241
}

impl Objects {
    fn new() -> Objects {
        Objects {
            map: [[DEFAULT; HEIGHT as usize]; WIDTH as usize],
            empties: ArrayVec::new(),
        }
    }

    pub fn get(&self, pos: PointUsize) -> Object {
        self.map[pos.x][pos.y]
    }

    pub fn empties_count(&self) -> usize {
        self.empties.len()
    }

    // Food

    pub fn set_food_on_empty(&mut self, pos: PointUsize) {
        self._remove_empty(pos);
        self.map[pos.x][pos.y] = FOOD;
    }

    pub fn set_food_on_body(&mut self, pos: PointUsize) {
        debug_assert!(self.is_body(pos));
        self.map[pos.x][pos.y] = FOOD;
    }

    pub fn init_food(&mut self, pos: PointUsize) {
        debug_assert!(self.is_default(pos));
        self.map[pos.x][pos.y] = FOOD;
    }

    pub fn is_food(&self, pos: PointUsize) -> bool {
        self.get(pos) == FOOD
    }

    // Body

    pub fn set_body_on_empty(&mut self, pos: PointUsize) {
        self._remove_empty(pos);
        self.map[pos.x][pos.y] = BODY;
    }

    pub fn set_body_on_food(&mut self, pos: PointUsize) {
        debug_assert!(self.is_food(pos));
        self.map[pos.x][pos.y] = BODY;
    }

    pub fn init_body(&mut self, pos: PointUsize) {
        debug_assert!(self.is_default(pos));
        self.map[pos.x][pos.y] = BODY;
    }

    pub fn is_body(&self, pos: PointUsize) -> bool {
        self.get(pos) == BODY
    }

    // Empty

    pub fn set_empty_on_body(&mut self, pos: PointUsize) {
        debug_assert!(self.is_body(pos), "At {:?} there is {} not BODY", pos, self.get(pos));
        self._set_empty(pos);
    }

    pub fn set_empty_on_food(&mut self, pos: PointUsize) {
        debug_assert!(self.is_food(pos));
        self._set_empty(pos);
    }

    pub fn init_empty(&mut self, pos: PointUsize) {
        debug_assert!(self.is_default(pos));
        self._set_empty(pos);
    }

    pub fn is_empty(&self, pos: PointUsize) -> bool {
        is_empty(self.get(pos))
    }

    fn _set_empty(&mut self, pos: PointUsize) {
        let empty = self.empties.len() as u8;
        self.empties.push(pos);
        self.map[pos.x][pos.y] = empty;
    }

    fn _remove_empty(&mut self, pos: PointUsize) {
        debug_assert!(self.is_empty(pos), "{:?} is not empty, it's {}", pos, self.get(pos));
        let i = self.map[pos.x][pos.y] as usize;
        self.empties.swap_remove(i);
        
        if i != self.empties.len() {
            // Now when where is new empty on this position
            // we must change map pointer
            let last_empty_pos = self.empties[i];
            self.map[last_empty_pos.x][last_empty_pos.y] = i as u8;
        }
    }

    pub fn get_empty_position(&self, rng: &mut impl rand::Rng) -> Option<PointUsize> {
        self.empties.choose(rng).copied()
    }

    // Default

    pub fn is_default(&self, pos: PointUsize) -> bool {
        self.map[pos.x][pos.y] == DEFAULT
    }

}

pub fn random_point_inside_borders() -> GridPoint {
    let mut rnd = thread_rng();
    Point {
        x: rnd.gen_range(0..WIDTH),
        y: rnd.gen_range(0..HEIGHT),
    }
}

const SNAKE_COLORS: [(u8, u8, u8); MAX_SNAKE_COUNT] = [
    (252, 90, 50),
    (50, 90, 252),
    (255, 250, 160),
    (105, 0, 198),
];
const HAZARD_COLOR: (u8, u8, u8) = (64, 64, 64);
const FOOD_CHAR: &str = "*";
// const DEAD_SNAKE_CHAR: &str = "X";

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut view: [[ColoredString; HEIGHT as usize]; WIDTH as usize] = Default::default();
        let mut output = String::new();

        for (i, snake) in self.snakes.iter().enumerate() {
            if !snake.is_alive() {
                continue;
            }
            let color = SNAKE_COLORS[i];

            output += &format!("{}: health {}; length {}\n", i, snake.health, snake.body.len());

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
            let fill = i.to_string().truecolor(color.0, color.1, color.2);
            
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

                if self.is_hazard(Point {x, y}) {
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

    pub fn head(&self) -> GridPoint {
        self.body[0]
    }

    pub fn tail(&self) -> GridPoint {
        self.body[self.body.len() - 1]
    }
}

impl Point<GridPoint> {
    pub const ZERO: Point<CoordType> = Point { x: 0, y: 0 };
}

impl<T> Add for Point<T> where T: Add<Output=T> {
    type Output = Point<T>;

    fn add(self, other: Point<T>) -> Point<T> {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

impl<T: AddAssign> AddAssign for Point<T> {
    fn add_assign(&mut self, other: Point<T>) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Into<PointUsize> for GridPoint {
    fn into(self) -> PointUsize {
        PointUsize {
            x: self.x as usize,
            y: self.y as usize,
        }
    }
}

impl From<PointUsize> for GridPoint {
    fn from(point: PointUsize) -> GridPoint {
        GridPoint {
            x: point.x as CoordType,
            y: point.y as CoordType,
        }
    }
}


impl Rectangle {
    pub fn contains(&self, p: GridPoint) -> bool {
        self.p0.x <= p.x && p.x < self.p1.x &&
        self.p0.y <= p.y && p.y < self.p1.y
    }

    #[allow(dead_code)]
    pub fn empty(&self) -> bool {
        self.p0.x >= self.p1.x ||
        self.p0.y >= self.p1.y
    }
}
