use log::info;

use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::{Duration, Instant};

use crate::api::objects::Movement;
use crate::engine::{EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::features::collector::Rewards;
use crate::game::{Board, MAX_SNAKE_COUNT};
use crate::mcts::search::Search;
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::{get_masks, get_random_actions_from_masks};
use crate::mcts::heuristics::flood_fill::flavored_flood_fill;

use super::config::SequentialMCTSConfig;

use super::ucb::UCB;
type Strategy = UCB;

struct Node {
    visits: f32,
    // Alive snakes
    agents: Vec<Agent>,
}

struct Agent {
    id: usize,
    strategy: Strategy,
}

impl Node {
    fn new(agents: Vec<Agent>) -> Node {
        Node {
            visits: 0.0,
            agents,
        }
    }
}

pub struct SequentialMCTS {
    config: SequentialMCTSConfig,
    nodes: HashMap<u64, RefCell<Node>, BuildHasherDefault<ZobristHasher>>,
}

impl Search for SequentialMCTS {
    fn search(&mut self, board: &Board, iterations_count: usize, _verbose: bool) {
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(board);
        }
    }

    fn search_with_time(&mut self, board: &Board, target_duration: Duration, verbose: bool) -> usize {
        let time_start = Instant::now();
        let time_end = time_start + target_duration;

        let mut i = 0;
        while Instant::now() < time_end {
            self.rollout(board);
            i += 1;
        }

        if verbose {
            let actual_duration = Instant::now() - time_start;
            self.print_stats(board);
            info!("Searched {} iterations in {} ms (target={} ms)", i, actual_duration.as_millis(), target_duration.as_millis());
        }
        
        i
    }

    fn get_final_movement(&self, board: &Board, agent_index: usize, verbose: bool) -> Movement {
        let node = self.nodes[&board.zobrist_hash.get_value()].borrow();
        let strategy = &node.agents[agent_index].strategy;
        strategy.get_final_movement()
    }

    fn shutdown(&self) {}
}

impl SequentialMCTS {
    pub fn new(config: SequentialMCTSConfig) -> SequentialMCTS {
        SequentialMCTS {
            nodes: HashMap::with_capacity_and_hasher(config.table_capacity, BuildHasherDefault::<ZobristHasher>::default()),
            config,
        }
    }

    #[allow(dead_code)]
    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(&board.zobrist_hash.get_value()).unwrap().borrow();

        for (i, agent) in node.agents.iter().enumerate() {
            info!("Snake {}", i);
            agent.strategy.print_stats(node.visits);
        }
    }

    fn rollout(&mut self, board: &Board) {
        let mut board = board.clone();
        let path = self.selection(&mut board);

        let masks = get_masks(&board);

        let rewards = self.simulation(&board);
        self.backpropagate(path, rewards);
        if !board.is_terminal() {
            self.expansion(&board, masks);
        }
    }

    fn selection(&self, board: &mut Board) -> Vec<(RefMut<Node>, [usize; MAX_SNAKE_COUNT])> {
        // let start = Instant::now();

        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        while path.len() < self.config.max_select_depth {
            let node_key = board.zobrist_hash.get_value();
            let node_option = self.nodes.get(&node_key);

            if node_option.is_none() {
                break
            }
            let node_cell = node_option.unwrap();

            let mut node = node_cell.borrow_mut();
            let n = node.visits;
            // TODO: MaybeUninit
            let mut joint_action = [0; MAX_SNAKE_COUNT];

            let mut alive_i = 0;

            for snake_i in 0..board.snakes.len() {
                if !board.snakes[snake_i].is_alive() {
                    continue;
                }
                let best_action = node.agents[alive_i].strategy.get_best_movement(n);

                joint_action[snake_i] = best_action;
                alive_i += 1;
            }

            advance_one_step_with_settings(
                board,
                &mut engine_settings,
                joint_action,
            );

            path.push((node, joint_action));
        }

        path
    }

    fn expansion(&mut self, board: &Board, masks: [[bool; 4]; MAX_SNAKE_COUNT]) {
        let mut agents = Vec::new();
        for i in 0..board.snakes.len() {
            if board.snakes[i].is_alive() {
                agents.push(Agent {
                    id: i,
                    strategy: Strategy::new(masks[i]),
                });
            }
        }

        let node = Node::new(agents);
        self.nodes.insert(board.zobrist_hash.get_value(), RefCell::new(node));
    }

    fn simulation(&self, board: &Board) -> Rewards {
        let mut board = board.clone();
        let rollout_cutoff = self.config.rollout_cutoff;

        let random = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let end_turn = board.turn + rollout_cutoff;
        while board.turn <= end_turn && !board.is_terminal() {
            let actions = get_random_actions_from_masks(random, &board);

            advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                actions,
            );
        }

        if board.snakes.iter().all(|snake| !snake.is_alive()) {
            return [self.config.draw_reward; MAX_SNAKE_COUNT];
        }

        let rewards = flavored_flood_fill(&board);
        // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
        rewards
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, [usize; MAX_SNAKE_COUNT])>, rewards: Rewards) {
        // let start = Instant::now();
        // info!("{:?}", self.nodes.keys());
        // info!("{:?}", path);
        for (mut node, joint_action) in path {
            // info!("{}", node_key);
            node.visits += 1.0;

            for agent in node.agents.iter_mut() {
                let reward = rewards[agent.id];
                let movement = joint_action[agent.id];
                agent.strategy.backpropagate(reward, movement);
            }
        }
    }

}
