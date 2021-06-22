use std::collections::VecDeque;
use std::ops::{Add, AddAssign};

use arrayvec::ArrayVec;

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

// TODO: Move everything below to engine crate

pub use crate::api::objects::Movement;

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

pub enum Action {
    // `DoNothing` allows freezing some snakes in places and while running others
    DoNothing,
    Move(Movement),
}

pub struct EngineSettings<'a, 'b> {
    // Can append elements to `board.food`, but must not mutate anything else.
    pub food_spawner: &'a mut dyn FnMut(&mut Board),

    // Can shrink `board.safe_zone`, but must not mutate anything else.
    pub safe_zone_shrinker: &'b mut dyn FnMut(&mut Board),
}

pub mod food_spawner {
    use super::*;
    use rand;

    fn spawn_one(rng: &mut impl rand::Rng, board: &mut Board) {
        let empty_squares_count = board.squares.data.iter().filter(|s| s.object == Object::Empty).count();

        if empty_squares_count == 0 {
            return
        }

        let needle = rng.gen_range(0..empty_squares_count);

        let mut i = 0;
        for x in 0..board.squares.len1 {
            for y in 0..board.squares.len2 {
                if let Object::Empty = board.squares[(x, y)].object {
                    if i == needle {
                        board.squares[(x, y)].object = Object::Food;
                        board.foods.push(Point { x: x as i32, y: y as i32 });
                        return;
                    }

                    i += 1;
                }
            }
        }

        unreachable!();
    }

    pub fn create_standard(mut rng: impl rand::Rng) -> impl FnMut(&mut Board) {
        move |board: &mut Board| {
            if board.foods.len() < 1 || rng.gen_ratio(20, 100) {
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
/// WIP
pub fn advance_one_step(
    board: &mut Board,
    engine_settings: &mut EngineSettings,
    snake_strategy: &mut dyn FnMut(/*snake_index:*/ usize, &Board) -> Action,
)
{
    board.turn += 1;

    let alive_snakes: ArrayVec<usize, MAX_SNAKE_COUNT> = (0..board.snakes.len())
        .filter(|&i| board.snakes[i].is_alive())
        .collect();

    debug_assert!(
        alive_snakes.iter().all(|&i| board.snakes[i].body.len() > 0)
    );

    // From https://docs.battlesnake.com/references/rules
    // 1. Each Battlesnake will have its chosen move applied:
    //     - A new body part is added to the board in the direction they moved.
    //     - Last body part (their tail) is removed from the board.
    //     - Health is reduced by 1.
    {
        let actions: ArrayVec<Action, MAX_SNAKE_COUNT> = alive_snakes.iter()
            .map(|&i| snake_strategy(i, &board))
            .collect();

        // TODO: bug with spawn_turn when snake is not moving. Is this field actually needed? It can be computed
        // separately, if needed
        for (i, action) in alive_snakes.iter().copied().zip(actions) {
            if let Action::Move(movement) = action {
                let snake = &mut board.snakes[i];
                debug_assert!(snake.body.len() > 0);

                snake.body.push_front(snake.body[0] + movement.to_direction());
                let old_tail = snake.body.pop_back().unwrap();
                snake.health -= 1;

                debug_assert_eq!(board.squares[old_tail].object, Object::BodyPart);

                board.squares[old_tail].object = Object::Empty;
                // TODO: wrong! this conflicts with next step of consuming food
                board.squares[snake.body[0]].object = Object::BodyPart;
            }
        }
    }

    // 2. Any Battlesnake that has found food will consume it:
    //     - Health reset set maximum.
    //     - Additional body part placed on top of current tail (this will extend their visible length by one on the
    //       next turn).
    //     - The food is removed from the board.
    {
        let mut eaten_food = ArrayVec::<Point, MAX_SNAKE_COUNT>::new();

        for i in alive_snakes.iter().copied() {
            let head = board.snakes[i].body[0];

            if board.squares[head].object != Object::Food {
                continue;
            }

            board.snakes[i].health = 100;

            let tail = *board.snakes[i].body.back().unwrap();
            board.snakes[i].body.push_back(tail);
            debug_assert_eq!(board.squares[tail].object, Object::BodyPart);
            eaten_food.push(head);
        }

        for food in eaten_food {
            board.foods.swap_remove(board.foods.iter().position(|&x| x == food).unwrap());
            board.squares[food].object = Object::Empty;
        }
    }

    // 3. Any new food spawning will be placed in empty squares on the board.
    {
        (engine_settings.food_spawner)(board);
    }

    // Battle Royale ruleset. Do in this order:
    // - Deal out-of-safe-zone damage
    // - Maybe shrink safe zone
    {
        for i in alive_snakes.iter().copied() {
            if !board.safe_zone.contains(board.snakes[i].body[0]) {
                board.snakes[i].health -= 15;
            }
        }

        (engine_settings.safe_zone_shrinker)(board);
    }

    // 4. Any Battlesnake that has been eliminated is removed from the game board:
    //     - Health less than or equal to 0
    //     - Moved out of bounds
    //     - Collided with themselves
    //     - Collided with another Battlesnake
    //     - Collided head-to-head and lost
    {
        let mut died_snakes = ArrayVec::<usize, MAX_SNAKE_COUNT>::new();

        for i in alive_snakes.iter().copied() {
            let snake = &board.snakes[i];

            let mut died = snake.health <= 0;
            died = died || !board.contains(snake.body[0]);
            died = died || matches!(board.squares[snake.body[0]].object, Object::BodyPart {..});
            if !died {
                for j in alive_snakes.iter().copied() {
                    if i != j && snake.body[0] == board.snakes[j].body[0] {
                        died = true;
                        break;
                    }
                }
            }

            if died {
                died_snakes.push(i);
            }
        }

        for i in died_snakes {
            board.snakes[i].health = 0;
            for p in board.snakes[i].body.iter().copied() {
                // TODO: Wait, this is wrong in case of body collision
                board.squares[p].object = Object::Empty;
            }
        }

        // In case we removed alive head, here we restore it.
        for i in alive_snakes.iter().copied() {
            if board.snakes[i].is_alive() {
                board.squares[board.snakes[i].body[0]].object = Object::BodyPart;
            }
        }
    }

}
