use std::env;

use balalaika::board_generator::generate_board;
use balalaika::mcts::utils::SearchOptions;
use mongodb::sync::Client;
use balalaika::engine::{food_spawner, EngineSettings, safe_zone_shrinker, advance_one_step_with_settings};
use balalaika::game::MAX_SNAKE_COUNT;
use balalaika::game_log::{save_game_log, GameLogBuilder};
use mcts::search::Search;
use mcts::utils::search;
use balalaika::mcts;
use mcts::seq::SequentialMCTS as MCTS;
use mcts::seq::SequentialMCTSConfig as MCTSConfig;

fn main() {
    let tag_option = env::var("SELFPLAY_TAG").ok();

    let uri = env::var("MONGO_URI").expect("Provide MONGO_URI");
    let client = Client::with_uri_str(uri).unwrap();

    let mcts_config = MCTSConfig::from_env();
    let search_options = SearchOptions::from_env();

    let mut engine_settings = EngineSettings {
        food_spawner: &mut food_spawner::create_standard,
        safe_zone_shrinker: &mut safe_zone_shrinker::standard,
    };

    loop {
        let mut board = generate_board();
        let mut game_log_builder = GameLogBuilder::new(
            board.snakes.clone(),
            board.safe_zone,
            &board.foods,
        );
        if let Some(tag) = &tag_option {
            game_log_builder.set_tag(tag.clone());
        }

        // Play game
        println!("Starting new game");
        while !board.is_terminal() {
            let mut mcts = MCTS::new(mcts_config.clone());
            search(&mut mcts, &board, search_options);

            let mut actions = [0; MAX_SNAKE_COUNT];

            let mut alive_i = 0;
            for snake_i in 0..board.snakes.len() {
                if !board.snakes[snake_i].is_alive() {
                    continue;
                }
                let action = mcts.get_final_movement(&board, alive_i, true);
                actions[snake_i] = action as usize;
                alive_i += 1;
            }
            advance_one_step_with_settings(&mut board, &mut engine_settings, actions);
            game_log_builder.add_turn_from_board(&board);
            // TODO: verbose
            println!("{}", board);
        }

        // Upload game
        let game_log = game_log_builder.finalize();
        save_game_log(&client, &game_log).expect("Error");
        println!("Saved game");
    }
}
