use std::collections::VecDeque;
use std::ops::{Add, AddAssign};

use crate::api;
use crate::vec2d::Vec2D;

pub struct Board {
    pub size: Point,
    pub food: Vec<Point>,
    pub snakes: Vec<Snake>,
    pub turn: u32, // TODO: i32 to allow negative turns?
    pub safe_zone: Rectangle,
    pub squares: Vec2D<Square>,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Object {
    Empty,
    Food,
    BodyPart {
        spawn_turn: i32,
    },
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

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

        let mut squares = Vec2D::init_same(
            board_api.width as usize,
            board_api.height as usize,
            Square {
                safe: true,
                object: Object::Empty,
            }
        );

        for api::objects::Point{x, y} in board_api.hazards.iter() {
            squares[(*x as usize, *y as usize)].safe = false;
        }

        Board {
            size: Point {
                x: board_api.width,
                y: board_api.height,
            },
            food: board_api.food.iter().map(Point::from_api).collect(),
            snakes: board_api.snakes.iter().map(Snake::from_api).collect(),
            turn: state_api.turn,
            safe_zone: Self::calcualate_safe_zone(&squares),
            squares: squares,
        }
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
            body: snake_api.body.iter().map(Point::from_api).collect(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

impl Point {
    pub fn from_api(point_api: &api::objects::Point) -> Point {
        Point { x: point_api.x, y: point_api.y }
    }
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

pub use crate::api::objects::Movement;

pub enum Action {
    // `DoNothing` allows freezing some snakes in places and while running others
    DoNothing,
    Move(Movement),
}

impl Movement {
    pub fn to_direction(self) -> Point {
        match self {
            Self::Right => Point {x:  1, y:  0},
            Self::Left  => Point {x: -1, y:  0},
            Self::Up    => Point {x:  0, y:  1},
            Self::Down  => Point {x:  0, y: -1},
        }
    }
}

pub struct EngineSettings<'a, 'b> {
    // Can append elements to `board.food`, but must not mutate anything else.
    pub food_spawner: &'a dyn FnMut(&mut Board),

    // Can shrink `board.safe_zone`, but must not mutate anything else.
    pub safe_zone_shrinker: &'b dyn FnMut(&mut Board),
}

pub mod food_spawner {
    use super::*;
    use rand;
    use rand::seq::SliceRandom;

    fn spawn_one(rng: &mut impl rand::Rng, board: &mut Board) {
        // TODO
    }

    pub fn create_standard(mut rng: impl rand::Rng) -> impl FnMut(&mut Board) {
        move |board: &mut Board| {
            if board.food.len() < 1 || rng.gen_ratio(20, 100) {
                spawn_one(&mut rng, board);
            }
        }
    }

    pub fn noop(_: &mut Board) {
    }
}

pub mod safe_zone_shrinker {
    use super::*;

    // pub fn standard

    pub fn noop(_: &mut Board) {
    }
}

/// Dead snakes are kept in array to preserve indices of all other snakes
pub fn advance_one_step(
    board: &mut Board,
    engine_settings: &EngineSettings,
    snake_strategy: &mut dyn FnMut(/*snake_index:*/ usize, &Board) -> Action,
)
{
    // From https://docs.battlesnake.com/references/rules
    // 1. Each Battlesnake will have its chosen move applied:
    //     - A new body part is added to the board in the direction they moved.
    //     - Last body part (their tail) is removed from the board.
    //     - Health is reduced by 1.
    let snakes_and_actions: Vec<(usize, Action)> = (0..board.snakes.len())
        .filter(|&i| board.snakes[i].is_alive())
        .map(|i| (i, snake_strategy(i, &board)))
        .collect();

    for (i, action) in snakes_and_actions {
        if let Action::Move(movement) = action {
            let snake = &mut board.snakes[i];

            if snake.is_alive() {
                assert!(snake.body.len() > 0);

                snake.body.push_front(snake.body[0] + movement.to_direction());
                snake.body.pop_back();
                snake.health -= 1;
            }
        }
    }

    // 2. Any Battlesnake that has found food will consume it:
    //     - Health reset set maximum.
    //     - Additional body part placed on top of current tail (this will extend their visible length by one on the
    //       next turn).
    //     - The food is removed from the board.

    // TODO

    // 3. Any new food spawning will be placed in empty squares on the board.

    // TODO

    // 4. Any Battlesnake that has been eliminated is removed from the game board:
    //     - Health less than or equal to 0
    //     - Moved out of bounds
    //     - Collided with themselves
    //     - Collided with another Battlesnake
    //     - Collided head-to-head and lost

    // TODO

    // 5. If there are enough Battlesnakes still present, repeat steps 1-5 for next turn.

    // TODO
}
