// use std::thread;
// use std::time::Duration;
// use crate::game::Board;
// use crate::mcts::{MCTS, MCTSConfig};

// pub fn root_parallelization(board: &Board, workers: usize, search_time: Duration, snake: usize) -> [u32; 4] {
//     let mcts_config = MCTSConfig::from_env();
//     let mut threads = Vec::with_capacity(workers);
    
//     for _i in 0..workers {
//         // TODO: Figure out how to pass reference to thread instead of copy.
//         // Thread can outlive board reference.
//         let board_clone = board.clone();
//         threads.push(thread::spawn(move || {
//             let mut mcts = MCTS::new(mcts_config);
//             mcts.search_with_time(&board_clone, search_time);
//             mcts.get_movement_visits(&board_clone, snake)
//         }));
//     }

//     let mut aggregated_visits = [0u32; 4];
//     for thread in threads {
//         let movement_visits = thread.join().unwrap();
//         for (movement, &visits) in movement_visits.iter().enumerate() {
//             aggregated_visits[movement] += visits;
//         }
//     }

//     aggregated_visits
// }
