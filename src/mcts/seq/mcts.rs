use rand::seq::SliceRandom;
use arrayvec::ArrayVec;

use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::{Duration, Instant};

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, MAX_SNAKE_COUNT};
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::get_masks;
use crate::mcts::bandit::MultiArmedBandit;
use crate::mcts::heuristics::flood_fill::flood_fill_estimate;

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
    visits: u32,
    // Alive snakes
    agents: Vec<Agent>,
}

struct Agent {
    id: usize,
    bandit: Strategy,
}

impl Node {
    fn new(agents: Vec<Agent>) -> Node {
        Node {
            visits: 0,
            agents: agents,
        }
    }

    fn get_agent_index(&self, agent_id: usize) -> usize {
        self.agents.iter().position(|a| a.id == agent_id).expect("There is no agent")
    }
}

pub struct SequentialMCTS {
    pub config: SequentialMCTSConfig,
    nodes: HashMap<u64, RefCell<Node>, BuildHasherDefault<ZobristHasher>>,
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

        for agent in node.agents.iter() {
            info!("Snake {}", agent.id);
            agent.bandit.print_stats(&self.config, node.visits);
        }
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(board);
        }
        self.print_stats(board);
    }

    pub fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
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

    fn rollout(&mut self, board: &Board) {
        let mut board = board.clone();
        let path = self.selection(&mut board);
        let rewards = self.simulation(board.clone());
        self.backpropagate(path, rewards);
        if !board.is_terminal() {
            self.expansion(&board);
        }
    }

    fn selection(&self, board: &mut Board) -> Vec<(RefMut<Node>, ArrayVec<(usize, Action), MAX_SNAKE_COUNT>)> {
        // let start = Instant::now();

        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let nodes = &self.nodes;

        while let Some(node_cell) = nodes.get(&board.zobrist_hash.get_value()) {
            let mut node = node_cell.borrow_mut();
            let n = node.visits;

            // TODO: This is ugly. Maybe change snake_strategy on simple arguments pass.
            let joint_action = advance_one_step_with_settings(
                board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let agent = &mut node.agents[agent_index];

                    let best_action = agent.bandit.get_best_movement(&self.config, n);
                    Action::Move(Movement::from_usize(best_action))
                }
            );

            path.push((node, joint_action));
        }

        path
    }

    fn expansion(&mut self, board: &Board) {
        let masks = get_masks(board);
        let agents = masks
            .into_iter()
            .map(|(agent_id, mask)| {
                Agent {
                    id: agent_id,
                    bandit: Strategy::new(mask)
                }
            })
            .collect();

        let node = Node::new(agents);
        self.nodes.insert(board.zobrist_hash.get_value(), RefCell::new(node));
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, ArrayVec<(usize, Action), MAX_SNAKE_COUNT>)>, rewards: Vec<f32>) {
        for (mut node, joint_action) in path {
            node.visits += 1;

            for (agent, Action::Move(movement)) in joint_action.into_iter() {
                let agent_index = node.get_agent_index(agent);

                let reward = rewards[agent];
                let movement = movement as usize;

                let agent = &mut node.agents[agent_index];
                agent.bandit.backpropagate(reward, movement);
            }
        }
    }

    fn simulation(&self, mut board: Board) -> Vec<f32> {
        let random = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let end_turn = board.turn + self.config.rollout_cutoff;
        while board.turn <= end_turn && !board.is_terminal() {
            let actions: HashMap<_, _> = get_masks(&board)
                .into_iter()
                .map(|(snake, movement_masks)| {
                    let &movement = movement_masks
                        .iter()
                        .enumerate()
                        .filter(|(_, &mask)| mask)
                        .map(|(movement, _)| movement)
                        .collect::<Vec<_>>()
                        .choose(random)
                        .unwrap_or(&0);
                    (snake, Action::Move(Movement::from_usize(movement)))
                })
                .collect();

            advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                &mut |snake, _| *actions.get(&snake).unwrap()
            );
        }

        let rewards = flood_fill_estimate(&board);
        let rewards = Vec::from(&rewards[0..board.snakes.len()]);

        // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
        rewards
    }

    pub fn get_final_movement(&self, board: &Board, agent: usize) -> Movement {
        let node = self.nodes[&board.zobrist_hash.get_value()].borrow();
        let agent_index = node.get_agent_index(agent);
        let agent = &node.agents[agent_index];

        agent.bandit.get_final_movement(&self.config, node.visits)
    }

    pub fn shutdown(&self) {}
}
