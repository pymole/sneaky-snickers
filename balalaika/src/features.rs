use crate::{
    game::{
        WIDTH,
        HEIGHT,
        Board,
        MAX_SNAKE_COUNT,
        Point,
        SIZE,
    },
    zobrist::{body_direction, BodyDirections}
};



// type BoolGrid = [[bool; HEIGHT as usize]; WIDTH as usize];

/// Bool grids:
/// - Food
/// - Hazard
/// - First player head
/// - Second player head
/// - First player body
/// - Second player body
/// - Body up
/// - Body right
/// - Body down
/// - Body left
/// 
/// Float players' parameters:
/// 1. First player health
/// 2. Second player health
// pub type Position = ([BoolGrid; 11], [f32; MAX_SNAKE_COUNT]);

// pub fn get_position(board: &Board) -> Position {
//     let w = WIDTH as usize;
//     let h = HEIGHT as usize;

//     let mut bool_grids: [BoolGrid; 11] = Default::default();
//     let mut float_parameters: [f32; MAX_SNAKE_COUNT] = Default::default();

//     // Hazards
//     {
//         for x in 0..w {
//             for y in 0..h {
//                 bool_grids[0][x][y] = board.is_hazard(Point{x, y});
//             }
//         }
//     }

//     // Snakes
//     let heads_i = 2;
//     let bodies_i = heads_i + MAX_SNAKE_COUNT;
//     let body_directions_i = bodies_i + MAX_SNAKE_COUNT;
//     for i in 0..MAX_SNAKE_COUNT {
//         let snake = &board.snakes[i];

//         float_parameters[i] = snake.health as f32 / 100.0;

//         let mut front = snake.head();
//         bool_grids[heads_i + i][front.x as usize][front.y as usize] = true;

//         for j in 1..snake.body.len() {
//             let back = snake.body[j];

//             bool_grids[bodies_i + i][back.x as usize][back.y as usize] = true;

//             let direction = body_direction(back, front);
//             if direction == BodyDirections::Still {
//                 // First two turn there can be body parts under each other.
//                 // They can't have direction.
//                 break;
//             }
//             bool_grids[body_directions_i + direction as usize][back.x as usize][back.y as usize] = true;

//             front = back;
//         }
//     }

//     // Food
//     let foods_i = body_directions_i + 4;
//     for food in board.foods.iter() {
//         bool_grids[foods_i][food.x as usize][food.y as usize] = true;
//     }

//     (bool_grids, float_parameters)
// }

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

pub fn get_nnue_features(board: &Board) -> [bool; TOTAL_FEATURES_SIZE] {
    let mut features = [false; TOTAL_FEATURES_SIZE];
    
    for (owner, snake) in board.snakes.iter().enumerate() {
        if !snake.is_alive() {
            continue;
        }
        
        // (owner[snakes], health[101])
        features[get_health_index(owner, snake.health)] = true;

        // (owner[snakes], head[grid])
        let head = snake.head();
        features[get_head_index(owner, head)] = true;
        
        // (owner[snakes], tail[grid])
        features[get_tail_index(owner, snake.tail())] = true;

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;

                let direction = body_direction(back, front);
                features[get_body_part_index(owner, back, direction)] = true;    
                break;
            }
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            features[get_body_part_index(owner, front, direction)] = true;
        }
        features[get_body_part_index(owner, head, BodyDirections::Still)] = true;

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = snake.body.len() - nearest_to_head_body_on_tail_index;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        features[get_bodies_on_tail_index(owner, bodies_on_tail)] = true;
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        features[get_food_index(food)] = true;
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                features[get_hazard_index(p)] = true;
            }
        }
    }

    features
}

fn get_point_index(p: Point) -> usize {
    (p.y * HEIGHT + p.x) as usize
}