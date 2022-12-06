use std::mem::{MaybeUninit, self};

use crate::{
    game::{
        WIDTH,
        HEIGHT,
        Board,
        MAX_SNAKE_COUNT,
        Point,
        SIZE, CENTER,
    },
    zobrist::{body_direction, BodyDirections}
};


pub type Rewards = ([f32; MAX_SNAKE_COUNT], bool);

pub fn get_rewards(board: &Board) -> Rewards {
    debug_assert!(board.is_terminal());
    let mut rewards = [0.0; MAX_SNAKE_COUNT];
    let mut draw = true;
    for (i, snake) in board.snakes.iter().enumerate() {
        if snake.is_alive() {
            draw = false;
            rewards[i] = 1.0;
            break;
        }
    }
    (rewards, draw)
}


// NNUE Features

const MAX_BODIES_ON_TAIL: usize = 3;
const ALL_PLAYERS_GRIDS_SIZE: usize = MAX_SNAKE_COUNT * SIZE;
const HEALTH_BARS: usize = 101;

const HEADS_AT: usize = 0;
const TAILS_AT: usize = ALL_PLAYERS_GRIDS_SIZE;
const BODY_PARTS_AT: usize = TAILS_AT + ALL_PLAYERS_GRIDS_SIZE;
const BODIES_ON_TAIL_AT: usize = BODY_PARTS_AT + ALL_PLAYERS_GRIDS_SIZE * 5;
const FOOD_AT: usize = BODIES_ON_TAIL_AT + MAX_BODIES_ON_TAIL * MAX_SNAKE_COUNT;
const HAZARD_AT: usize = FOOD_AT + SIZE;
const HEALTH_AT: usize = HAZARD_AT + SIZE;
pub const TOTAL_FEATURES_SIZE: usize = HEALTH_AT + MAX_SNAKE_COUNT * HEALTH_BARS;


const fn calculate_owner_heads_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_heads_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_heads_at[i] = HEADS_AT + i * SIZE;
        i += 1;
    }
    owner_heads_at
}

const fn calculate_owner_tails_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_tails_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_tails_at[i] = TAILS_AT + i * SIZE;
        i += 1;
    }
    owner_tails_at
}

const fn calculate_owner_body_parts_at() -> [[usize; 5]; MAX_SNAKE_COUNT] {
    let mut owner_body_parts_at = [[0; 5]; MAX_SNAKE_COUNT];
    let mut i = 0;
    let mut d;
    let mut owner_offset;
    while i < MAX_SNAKE_COUNT {
        d = 0;
        owner_offset = BODY_PARTS_AT + i * SIZE * 5;
        while d < 5 {
            owner_body_parts_at[i][d] = owner_offset + d * SIZE;
            d += 1;
        }
        i += 1;
    }
    owner_body_parts_at
}

const fn calculate_owner_bodies_on_tail_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_bodies_on_tail_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_bodies_on_tail_at[i] = BODIES_ON_TAIL_AT + i * MAX_BODIES_ON_TAIL;
        i += 1;
    }
    owner_bodies_on_tail_at
}

const fn calculate_owner_health_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_on_health_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_on_health_at[i] = HEALTH_AT + i * HEALTH_BARS;
        i += 1;
    }
    owner_on_health_at
}

const OWNER_HEADS_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_heads_at();
const OWNER_TAILS_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_tails_at();
const OWNER_BODY_PARTS_AT: [[usize; 5]; MAX_SNAKE_COUNT] = calculate_owner_body_parts_at();
const OWNER_BODIES_ON_TAIL_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_bodies_on_tail_at();
const OWNER_HEALTH_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_health_at();

fn get_head_index(owner: usize, head: Point) -> usize {
    OWNER_HEADS_AT[owner] + get_point_index(head)
}

fn get_tail_index(owner: usize, tail: Point) -> usize {
    OWNER_TAILS_AT[owner] + get_point_index(tail)
}

fn get_body_part_index(owner: usize, body: Point, body_direction: BodyDirections) -> usize {
    OWNER_BODY_PARTS_AT[owner][body_direction as usize] + get_point_index(body)
}

fn get_bodies_on_tail_index(owner: usize, bodies_on_tail: usize) -> usize {
    OWNER_BODIES_ON_TAIL_AT[owner] + bodies_on_tail - 1
}

fn get_food_index(food: Point) -> usize {
    FOOD_AT + get_point_index(food)
}

fn get_hazard_index(hazard: Point) -> usize {
    HAZARD_AT + get_point_index(hazard)
}

fn get_health_index(owner: usize, health: i32) -> usize {
    OWNER_HEALTH_AT[owner] + health as usize
}

pub fn collect_features_loop(board: &Board, collector: &mut impl Collector) {
    for (owner, snake) in board.snakes.iter().enumerate() {
        if !snake.is_alive() {
            continue;
        }
        
        // (owner[snakes], health[101])
        collector.health(owner, snake.health);

        // (owner[snakes], head[grid])
        let head = snake.head();
        collector.head(owner, head);
        
        // (owner[snakes], tail[grid])
        collector.head(owner, snake.tail());

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;

                let direction = body_direction(back, front);
                collector.body_direction(owner, back, direction);
                break;
            }
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            collector.body_direction(owner, front, direction);
        }
        collector.body_direction(owner, head, BodyDirections::Still);

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = snake.body.len() - nearest_to_head_body_on_tail_index;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        collector.bodies_on_tail(owner, bodies_on_tail);
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        collector.food(food);
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                collector.hazard(p);
            }
        }
    }
}

pub trait Collector {
    fn health(&mut self, owner: usize, health: i32);
    fn head(&mut self, owner: usize, pos: Point);
    fn tail(&mut self, owner: usize, pos: Point);
    fn body_direction(&mut self, owner: usize, body: Point, body_direction: BodyDirections);
    fn bodies_on_tail(&mut self, owner: usize, bodies_on_tail: usize);
    fn food(&mut self, pos: Point);
    fn hazard(&mut self, pos: Point);
}

#[derive(Default)]
pub struct IndicesCollector {
    pub indices: Vec<usize>,
}

impl IndicesCollector {
    pub fn new() -> IndicesCollector {
        IndicesCollector { indices: Vec::new() }
    }
}

impl Collector for IndicesCollector {
    fn health(&mut self, owner: usize, health: i32) {
        self.indices.push(get_health_index(owner, health));
    }

    fn head(&mut self, owner: usize, pos: Point) {
        self.indices.push(get_head_index(owner, pos));
    }

    fn tail(&mut self, owner: usize, pos: Point) {
        self.indices.push(get_tail_index(owner, pos));
    }

    fn body_direction(&mut self, owner: usize, body: Point, body_direction: BodyDirections) {
        self.indices.push(get_body_part_index(owner, body, body_direction));
    }

    fn bodies_on_tail(&mut self, owner: usize, bodies_on_tail: usize) {
        self.indices.push(get_bodies_on_tail_index(owner, bodies_on_tail));
    }

    fn food(&mut self, pos: Point) {
        self.indices.push(get_food_index(pos));
    }

    fn hazard(&mut self, pos: Point) {
        self.indices.push(get_hazard_index(pos));
    }
}

pub struct FlipRotateIndicesCollector {
    pub collectors: [IndicesCollector; 16]
}

impl FlipRotateIndicesCollector {
    pub fn new() -> FlipRotateIndicesCollector {
        FlipRotateIndicesCollector {
            collectors: Default::default(),
        }
    }
}

impl Collector for FlipRotateIndicesCollector {
    fn health(&mut self, owner: usize, health: i32) {
        self.collectors.iter_mut().for_each(|collector| collector.health(owner, health));
    }

    fn head(&mut self, owner: usize, pos: Point) {
        for (i, flip_rot) in flip_rotate_point(pos).into_iter().enumerate() {
            self.collectors[i].head(owner, flip_rot);
        }
    }

    fn tail(&mut self, owner: usize, pos: Point) {
        for (i, flip_rot) in flip_rotate_point(pos).into_iter().enumerate() {
            // TODO: Manually calculate index once
            self.collectors[i].tail(owner, flip_rot);
        }
    }

    fn body_direction(&mut self, owner: usize, body: Point, body_direction: BodyDirections) {
        flip_rotate_point(body)
            .into_iter()
            .zip(flip_rotate_body_direction(body_direction).into_iter())
            .enumerate()
            .for_each(|(i, (body, body_direction))| {
                self.collectors[i].body_direction(owner, body, body_direction);
            })
    }

    fn bodies_on_tail(&mut self, owner: usize, bodies_on_tail: usize) {
        self.collectors.iter_mut().for_each(|collector| collector.bodies_on_tail(owner, bodies_on_tail));
    }

    fn food(&mut self, pos: Point) {
        for (i, flip_rot) in flip_rotate_point(pos).into_iter().enumerate() {
            self.collectors[i].food(flip_rot);
        }
    }

    fn hazard(&mut self, pos: Point) {
        for (i, flip_rot) in flip_rotate_point(pos).into_iter().enumerate() {
            self.collectors[i].hazard(flip_rot);
        }
    }
}

fn get_point_index(p: Point) -> usize {
    (p.y * HEIGHT + p.x) as usize
}

fn flip_point(p: Point) -> [Point; 4] {
    [
        Point {x: p.x, y: p.y},                             // original
        Point {x: WIDTH - p.x - 1, y: p.y},                 // x-flip
        Point {x: p.x, y: HEIGHT - p.y - 1},                // y-flip
        Point {x: WIDTH - p.x - 1, y: HEIGHT - p.y - 1},    // x and y flip
    ]
}

fn rotate_point(p: Point) -> [Point; 4] {
    let diff = Point {x: p.x - CENTER.x, y: p.y - CENTER.y};
    [
        Point {x: p.x, y: p.y},
        Point {x: CENTER.x + 0 * diff.x + 1 * diff.y, y: CENTER.y - 1 * diff.x + 0 * diff.y},  // Units x=(0, -1); y=(1, 0)
        Point {x: CENTER.x - 1 * diff.x + 0 * diff.y, y: CENTER.y + 0 * diff.x - 1 * diff.y},  // Units x=(-1, 0); y=(0, -1)
        Point {x: CENTER.x + 0 * diff.x - 1 * diff.y, y: CENTER.y + 1 * diff.x + 0 * diff.y},  // Units x=(0, 1); y=(-1, 0)
    ]
}

fn flip_rotate_point(pos: Point) -> [Point; 16] {
    
    let mut points: [MaybeUninit<Point>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
    for (rot_i, rot) in rotate_point(pos).into_iter().enumerate() {
        for (flip_i, flip_rot) in flip_point(rot).into_iter().enumerate() {
            points[rot_i * 4 + flip_i] = MaybeUninit::new(flip_rot);
        }
    }
    unsafe { mem::transmute(points) }
}

fn flip_body_direction(body_direction: BodyDirections) -> [BodyDirections; 4] {
    if body_direction == BodyDirections::Still {
        return [BodyDirections::Still; 4];
    }
    let b = body_direction as usize;
    [
        BodyDirections::from(b),                                    // original
        BodyDirections::from((b + 2 * (1 - (b + 1) % 2)) % 4),      // x-flip
        BodyDirections::from((b + 2 * ((b + 1) % 2)) % 4),          // y-flip
        BodyDirections::from((b + 2) % 4),                          // x and y flip
    ]
}

fn rotate_body_direction(body_direction: BodyDirections) -> [BodyDirections; 4] {
    if body_direction == BodyDirections::Still {
        return [BodyDirections::Still; 4];
    }
    let b = body_direction as usize;
    [
        BodyDirections::from(b),
        BodyDirections::from((b + 1) % 4),
        BodyDirections::from((b + 2) % 4),
        BodyDirections::from((b + 3) % 4),
    ]
}

fn flip_rotate_body_direction(body_direction: BodyDirections) -> [BodyDirections; 16] {
    let mut body_directions: [MaybeUninit<BodyDirections>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
    for (rot_i, rot) in rotate_body_direction(body_direction).into_iter().enumerate() {
        for (flip_i, flip_rot) in flip_body_direction(rot).into_iter().enumerate() {
            body_directions[rot_i * 4 + flip_i] = MaybeUninit::new(flip_rot);
        }
    }
    unsafe { mem::transmute(body_directions) }
}

#[cfg(test)]
mod tests {
    use crate::{
        zobrist::BodyDirections,
        features::{flip_body_direction, rotate_body_direction, rotate_point, flip_point}, game::Point
    };

    #[test]
    fn test_rotate_point() {
        assert_eq!(rotate_point(Point {x: 10, y: 10}), [Point {x: 10, y: 10}, Point {x: 10, y: 0}, Point {x: 0, y: 0}, Point {x: 0, y: 10}]);
        assert_eq!(rotate_point(Point {x: 9, y: 2}), [Point {x: 9, y: 2}, Point {x: 2, y: 1}, Point {x: 1, y: 8}, Point {x: 8, y: 9}]);
        assert_eq!(rotate_point(Point {x: 5, y: 5}), [Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}]);
    }

    #[test]
    fn test_flip_point() {
        assert_eq!(flip_point(Point {x: 10, y: 10}), [Point {x: 10, y: 10}, Point {x: 0, y: 10}, Point {x: 10, y: 0}, Point {x: 0, y: 0}]);
        assert_eq!(flip_point(Point {x: 9, y: 2}), [Point {x: 9, y: 2}, Point {x: 1, y: 2}, Point {x: 9, y: 8}, Point {x: 1, y: 8}]);
        assert_eq!(flip_point(Point {x: 5, y: 5}), [Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}]);
    }

    #[test]
    fn test_flip_body_direction() {
        assert_eq!(flip_body_direction(BodyDirections::Up), [BodyDirections::Up, BodyDirections::Up, BodyDirections::Down, BodyDirections::Down]);
        assert_eq!(flip_body_direction(BodyDirections::Right), [BodyDirections::Right, BodyDirections::Left, BodyDirections::Right, BodyDirections::Left]);
        assert_eq!(flip_body_direction(BodyDirections::Down), [BodyDirections::Down, BodyDirections::Down, BodyDirections::Up, BodyDirections::Up]);
        assert_eq!(flip_body_direction(BodyDirections::Left), [BodyDirections::Left, BodyDirections::Right, BodyDirections::Left, BodyDirections::Right]);
        assert_eq!(flip_body_direction(BodyDirections::Still), [BodyDirections::Still, BodyDirections::Still, BodyDirections::Still, BodyDirections::Still]);
    }

    #[test]
    fn test_rotate_body_direction() {
        assert_eq!(rotate_body_direction(BodyDirections::Up), [BodyDirections::Up, BodyDirections::Right, BodyDirections::Down, BodyDirections::Left]);
        assert_eq!(rotate_body_direction(BodyDirections::Right), [BodyDirections::Right, BodyDirections::Down, BodyDirections::Left, BodyDirections::Up]);
        assert_eq!(rotate_body_direction(BodyDirections::Down), [BodyDirections::Down, BodyDirections::Left, BodyDirections::Up, BodyDirections::Right]);
        assert_eq!(rotate_body_direction(BodyDirections::Left), [BodyDirections::Left, BodyDirections::Up, BodyDirections::Right, BodyDirections::Down]);
        assert_eq!(rotate_body_direction(BodyDirections::Still), [BodyDirections::Still, BodyDirections::Still, BodyDirections::Still, BodyDirections::Still]);    
    }
}