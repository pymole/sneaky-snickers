use crate::game::{Point, Object, MAX_SNAKE_COUNT};

const MAX_WIDTH: usize = 11;
const MAX_HEIGHT: usize = 11;

type ValueInt = i128;

lazy_static! {
    static ref SNAKE_BODIES: [[[[ValueInt; 2]; MAX_SNAKE_COUNT]; MAX_HEIGHT]; MAX_WIDTH] = {
        let mut snake_bodies = [[[[0; 2]; MAX_SNAKE_COUNT]; MAX_HEIGHT]; MAX_WIDTH];
        for x in 0..MAX_WIDTH {
            for y in 0..MAX_HEIGHT {
                for player in 0..MAX_SNAKE_COUNT {
                    // Head, body, eat
                    for piece in 0..2 {
                        snake_bodies[x][y][player][piece] = rand::random();
                    }
                }
            }
        }
        snake_bodies
    };

    static ref FOOD: [[ValueInt; MAX_WIDTH]; MAX_HEIGHT] = {
        let mut food = [[0; MAX_HEIGHT]; MAX_WIDTH];
        for x in 0..MAX_WIDTH {
            for y in 0..MAX_HEIGHT {
                food[x][y] = rand::random();
            }
        }
        food
    };
}


#[derive(PartialEq, Debug, Clone, Copy, Eq)]
pub struct ZobristHash {
    value: ValueInt,
}

impl ZobristHash {
    pub fn new() -> ZobristHash {
        ZobristHash {
            value: 0,
        }
    }

    pub fn xor_snake_part(&mut self, point: Point, player: usize, put_object: Object) {
        if put_object == Object::Food || put_object == Object::Empty {
            unreachable!("Put only snake parts (head or body)");
        }
        self.value ^= SNAKE_BODIES[point.x as usize][point.y as usize][player][put_object as usize];
    }

    pub fn xor_snake_head(&mut self, point: Point, player: usize) {
        self.xor_snake_part(point, player, Object::Head);
    }

    pub fn xor_snake_body_part(&mut self, point: Point, player: usize) {
        self.xor_snake_part(point, player, Object::BodyPart);
    }

    pub fn xor_food(&mut self, point: Point) {
        self.value ^= FOOD[point.x as usize][point.y as usize];
    }

    pub fn xor_turn(&mut self, turn: i32) {
        // Spiral hazard included in turn
        self.value ^= turn as ValueInt;
    }

    pub fn get_value(&self) -> ValueInt {
        self.value
    }
}
