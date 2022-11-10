use std::{collections::VecDeque, env};

use mongodb::sync::Client;
use sneaky_snickers::engine::food_spawner;
use sneaky_snickers::game::MAX_SNAKE_COUNT;
use sneaky_snickers::game_log::{GameLog, save_game_log};
use sneaky_snickers::mcts::config::Config;
use sneaky_snickers::mcts::search::Search;
use sneaky_snickers::mcts::utils::search;
use sneaky_snickers::{mcts};
use sneaky_snickers::{game::{Board, Snake, random_point_inside_borders}, engine::advance_one_step};

cfg_if::cfg_if! {
    if #[cfg(feature = "par")] {
        use mcts::parallel::ParallelMCTS as MCTS;
        use mcts::parallel::ParallelMCTSConfig as MCTSConfig;
    } else {
        use mcts::seq::SequentialMCTS as MCTS;
        use mcts::seq::SequentialMCTSConfig as MCTSConfig;
    }
}


fn main() {
    let uri = env::var("MONGO_URI").expect("Provide MONGO_URI");
    let client = Client::with_uri_str(uri).unwrap();

    let mcts_config = MCTSConfig::from_env();

    loop {
        // Generate board
        let head1 = random_point_inside_borders();
        let head2 = loop {
            let p = random_point_inside_borders();
            if p != head1 {
                break p
            }
        };

        let snakes = [
            Snake {
                health: 100,
                body: VecDeque::from([head1, head1, head1]),
            },
            Snake {
                health: 100,
                body: VecDeque::from([head2, head2, head2]),
            },
        ];

        let foods = vec![];
        let hazard_start = random_point_inside_borders();
        let hazards = vec![];

        let mut board = Board::new(
            0,
            foods,
            Some(hazard_start),
            None,
            snakes,
        );

        let food_count = 3;
        for _ in 0..food_count {
            food_spawner::create_standard(&mut board);
        }

        let mut game_log = GameLog::new(
            board.snakes.clone(),
            &hazards,
            &board.foods,
        );

        // Play game
        println!("Starting new game");
        while !board.is_terminal() {
            let mut mcts = MCTS::new(mcts_config.clone());
            search(&mut mcts, &board);

            let mut actions = [0; MAX_SNAKE_COUNT];

            for snake_index in 0..board.snakes.len() {
                let action = mcts.get_final_movement(&board, snake_index);
                actions[snake_index] = action as usize;
            }
            advance_one_step(&mut board, actions);
            game_log.add_turn_from_board(&board);
        }

        // Upload game
        save_game_log(&client, &game_log);
        println!("Saved game");
    }

}
