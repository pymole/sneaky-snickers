use log::info;
use rand::seq::SliceRandom;

use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::mem::{MaybeUninit, self};
use std::time::{Duration, Instant};

use crate::api::objects::Movement;
use crate::engine::{EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, MAX_SNAKE_COUNT};
use crate::mcts::search::Search;
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::get_masks;
use crate::mcts::bandit::MultiArmedBandit;
use crate::mcts::heuristics::flood_fill::flood_fill;

use super::config::SequentialMCTSConfig;

cfg_if::cfg_if! {
    if #[cfg(feature = "ts")] {
        use super::thompson::ThompsonSampling;
        type Strategy = ThompsonSampling;
    } else {
        use super::ucb::UCB;
        type Strategy = UCB;
    }
}

struct Node {
    visits: f32,
    // Alive snakes
    agents: [Strategy; MAX_SNAKE_COUNT],
}

impl Node {
    fn new(agents: [Strategy; MAX_SNAKE_COUNT]) -> Node {
        Node {
            visits: 0.0,
            agents: agents,
        }
    }
}

pub struct SequentialMCTS {
    config: SequentialMCTSConfig,
    nodes: HashMap<u64, RefCell<Node>, BuildHasherDefault<ZobristHasher>>,
}

impl Search<SequentialMCTSConfig> for SequentialMCTS {
    fn search(&mut self, board: &Board, iterations_count: usize) {
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(board);
        }
        self.print_stats(board);
    }

    fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
        let time_start = Instant::now();
        let time_end = time_start + target_duration;

        let mut i = 0u32;
        while Instant::now() < time_end {
            self.rollout(board);
            i += 1;
        }

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", i, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn get_final_movement(&self, board: &Board, agent_index: usize) -> Movement {
        let node = self.nodes[&board.zobrist_hash.get_value()].borrow();
        let bandit = &node.agents[agent_index];
        bandit.get_final_movement(&self.config, node.visits)
    }

    fn get_config(&self) -> &SequentialMCTSConfig {
        &self.config
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

    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(&board.zobrist_hash.get_value()).unwrap().borrow();

        for (i, bandit) in node.agents.iter().enumerate() {
            info!("Snake {}", i);
            bandit.print_stats(&self.config, node.visits);
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
            let mut joint_action = [0; MAX_SNAKE_COUNT];

            for agent_index in 0..board.snakes.len() {
                let bandit = &mut node.agents[agent_index];
                let best_action = bandit.get_best_movement(&self.config, n);

                joint_action[agent_index] = best_action;
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
        let agents: [Strategy; MAX_SNAKE_COUNT] = {
            let mut agents: [MaybeUninit<Strategy>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
            for i in 0..board.snakes.len() {
                agents[i] = MaybeUninit::new(Strategy::new(masks[i]));
            }
            unsafe { mem::transmute(agents) }
        };

        let node = Node::new(agents);
        self.nodes.insert(board.zobrist_hash.get_value(), RefCell::new(node));
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, [usize; MAX_SNAKE_COUNT])>, rewards: [f32; MAX_SNAKE_COUNT]) {
        // let start = Instant::now();
        // info!("{:?}", self.nodes.keys());
        // info!("{:?}", path);
        for (mut node, joint_action) in path {
            // info!("{}", node_key);
            node.visits += 1.0;

            for (agent_index, bandit) in node.agents.iter_mut().enumerate() {
                let reward = rewards[agent_index];
                let movement = joint_action[agent_index];

                bandit.backpropagate(reward, movement);
            }
        }
    }

    fn simulation(&self, board: &Board) -> [f32; MAX_SNAKE_COUNT] {
        let mut board = board.clone();
        let rollout_cutoff = self.config.rollout_cutoff;

        let random = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let end_turn = board.turn + rollout_cutoff;
        while board.turn <= end_turn && !board.is_terminal() {
            let mut actions = [0; MAX_SNAKE_COUNT];
            let masks = get_masks(&board);

            for snake_index in 0..board.snakes.len() {
                let &movement = masks[snake_index]
                    .iter()
                    .enumerate()
                    .filter(|(_, &mask)| mask)
                    .map(|(movement, _)| movement)
                    .collect::<Vec<_>>()
                    .choose(random)
                    .unwrap_or(&0);
                actions[snake_index] = movement;
            }

            advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                actions,
            );
        }

        if board.snakes.iter().all(|snake| !snake.is_alive()) {
            return [self.config.draw_reward; MAX_SNAKE_COUNT];
        }

        let len_sum: f32 = board.snakes.iter().map(|snake| snake.body.len() as f32).sum();

        let mut rewards = flood_fill(&board);

        for i in 0..board.snakes.len() {
            rewards[i] *= board.snakes[i].body.len() as f32 / len_sum;
        }
        // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
        rewards
    }
}
