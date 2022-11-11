use std::{collections::VecDeque, env};

use mongodb::sync::Client;
use balalaika::engine::{food_spawner, EngineSettings, safe_zone_shrinker, advance_one_step_with_settings};
use balalaika::game::{MAX_SNAKE_COUNT, Board, Snake, random_point_inside_borders};
use balalaika::game_log::{save_game_log, GameLogBuilder, rewind};
use mcts::config::Config;
use mcts::search::Search;
use mcts::utils::search;
use balalaika::mcts;

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

    let mut engine_settings = EngineSettings {
        food_spawner: &mut food_spawner::create_standard,
        safe_zone_shrinker: &mut safe_zone_shrinker::standard,
    };

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

        food_spawner::create_standard(&mut board);

        let mut game_log_builder = GameLogBuilder::new(
            board.snakes.clone(),
            &hazards,
            &board.foods,
        );
        game_log_builder.add_tag("selfplay-v1.0.0".to_string());
        game_log_builder.set_hazard_start(board.hazard_start);

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
            advance_one_step_with_settings(&mut board, &mut engine_settings, actions);
            game_log_builder.add_turn_from_board(&board);
        }

        // Upload game
        let game_log = game_log_builder.finalize();
        rewind(&game_log);
        save_game_log(&client, &game_log);
        println!("Saved game");
    }

}
