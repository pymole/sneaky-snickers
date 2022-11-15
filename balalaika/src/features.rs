use crate::{
    game::{
        WIDTH,
        HEIGHT,
        Board,
        MAX_SNAKE_COUNT,
    },
    zobrist::{body_direction, BodyDirections}
};



type BoolGrid = [[bool; HEIGHT as usize]; WIDTH as usize];

/// Bool grids:
/// - Food
/// - Hazard
/// - Hazard start
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
pub type Position = ([BoolGrid; 11], [f32; MAX_SNAKE_COUNT]);

pub fn get_position(board: &Board) -> Position {
    let w = WIDTH as usize;
    let h = HEIGHT as usize;

    let mut bool_grids: [BoolGrid; 11] = Default::default();
    let mut float_parameters: [f32; MAX_SNAKE_COUNT] = Default::default();

    // Hazards
    {
        let i = 0;
        for x in 0..w {
            for y in 0..h {
                bool_grids[i][x][y] = board.hazard[(x, y)];
            }
        }
    }

    // Hazard start
    {
        let i = 1;
        bool_grids[i][board.hazard_start.x as usize][board.hazard_start.y as usize] = true;
    }

    // Snakes
    let heads_i = 2;
    let bodies_i = heads_i + MAX_SNAKE_COUNT;
    let body_directions_i = bodies_i + MAX_SNAKE_COUNT;
    for i in 0..MAX_SNAKE_COUNT {
        let snake = &board.snakes[i];

        float_parameters[i] = snake.health as f32 / 100.0;

        let mut front = snake.head();
        bool_grids[heads_i + i][front.x as usize][front.y as usize] = true;

        for j in 1..snake.body.len() {
            let back = snake.body[j];

            bool_grids[bodies_i + i][back.x as usize][back.y as usize] = true;

            let direction = body_direction(back, front);
            if direction == BodyDirections::Still {
                // First two turn there can be body parts under each other.
                // They can't have direction.
                break;
            }
            bool_grids[body_directions_i + direction as usize][back.x as usize][back.y as usize] = true;

            front = back;
        }
    }

    // Food
    let foods_i = body_directions_i + 4;
    for food in board.foods.iter() {
        bool_grids[foods_i][food.x as usize][food.y as usize] = true;
    }

    (bool_grids, float_parameters)
}

pub type Rewards = ([f32; MAX_SNAKE_COUNT], bool);

pub fn get_rewards(board: &Board) -> Rewards {
    debug_assert!(board.is_terminal());
    let mut rewards = [0.0; MAX_SNAKE_COUNT];
    let mut draw = true;
    for (i, snake) in board.snakes.iter().enumerate() {
        let is_alive = snake.is_alive();
        if is_alive {
            draw = false;
            rewards[i] = 1.0;
        }
    }
    (rewards, draw)
}