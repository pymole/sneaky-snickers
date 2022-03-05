use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::{Duration, Instant};
use std::cell::{RefCell, RefMut};
use std::sync::Arc;
use rand::seq::SliceRandom;

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Snake};
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::get_masks;
use crate::mcts::bandit::MultiArmedBandit;

use super::bu_uct::BUUCT;
use super::config::ParallelMCTSConfig;
use super::pool::{Pool, Task};

type Strategy = BUUCT;

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

pub struct ParallelMCTS {
    pub config: ParallelMCTSConfig,
    nodes: HashMap<u64, Node, BuildHasherDefault<ZobristHasher>>,
    simulation_pool: Pool<(usize, Vec<f32>)>,
    iterations: usize,
}


impl ParallelMCTS {
    pub fn new(config: ParallelMCTSConfig) -> ParallelMCTS {
        let simulation_pool = Pool::<(usize, Vec<f32>)>::new(config.simulation_workers);

        ParallelMCTS {
            nodes: HashMap::with_capacity_and_hasher(config.table_capacity, BuildHasherDefault::<ZobristHasher>::default()),
            config,
            simulation_pool,
            iterations: 0,
        }
    }

    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();

        for agent in node.agents.iter() {
            info!("Snake {}", agent.id);
            agent.bandit.print_stats(&self.config, node.visits);
        }
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        let mut simulation_contexts = HashMap::with_capacity(self.config.simulation_workers);

        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(board, &mut simulation_contexts);
            self.iterations += 1;
        }
        self.print_stats(board);
    }

    pub fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
        let mut simulation_contexts = HashMap::with_capacity(self.config.simulation_workers);

        let time_start = Instant::now();
        let time_end = time_start + target_duration;

        while Instant::now() < time_end {
            self.rollout(board, &mut simulation_contexts);
            self.iterations += 1;
        }

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", self.iterations, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn rollout(&mut self, board: &Board, simulation_contexts: &mut HashMap<usize, Vec<(u64, Vec<(usize, Action)>)>>) {
        // let start = Instant::now();
        let path = self.selection(board.clone());

        if !board.is_terminal() {
            self.expansion(&board);
        }
        let key = self.iterations.clone();
        simulation_contexts.insert(key, path);

        self.simulation(&board);

        if self.simulation_pool.tasks_count() < self.config.simulation_workers {
            return;
        }

        let (simulation_id, rewards) = self.simulation_pool.wait_result();
        let path = simulation_contexts.remove(&simulation_id).unwrap();

        self.backpropagate(path, rewards);
    }

    fn selection(&mut self, mut board: Board) -> Vec<(u64, Vec<(usize, Action)>)> {
        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let config = &self.config;

        while let Some(node) = self.nodes.get_mut(&board.zobrist_hash.get_value()) {
            let n = node.visits;

            // TODO: This is ugly. Maybe change snake_strategy on simple arguments pass.
            let joint_action = advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let agent = &mut node.agents[agent_index];

                    let best_action = agent.bandit.get_best_movement(config, n);
                    Action::Move(Movement::from_usize(best_action))
                }
            );

            path.push((board.zobrist_hash.get_value(), joint_action));
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
        self.nodes.insert(board.zobrist_hash.get_value(), node);
    }

    fn simulation(&mut self, board: &Board) {
        let mut board = board.clone();
        let rollout_cutoff = self.config.rollout_cutoff;
        let beta = self.config.rollout_beta;
        let draw_reward = self.config.draw_reward;

        let simulation_id = self.iterations;

        let task = Task {
            fn_box: Box::new(move || {
                let random = &mut rand::thread_rng();
    
                let mut engine_settings = EngineSettings {
                    food_spawner: &mut food_spawner::noop,
                    safe_zone_shrinker: &mut safe_zone_shrinker::standard,
                };
    
                let end_turn = board.turn + rollout_cutoff;
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
                
                let alive_count = board.snakes.iter().filter(|snake| snake.is_alive()).count();
                let health_norm : f32 = board.snakes.iter().map(|snake| (snake.health as f32).powf(beta)).sum();
    
                let reward = |snake: &Snake|
                    if alive_count == 0 { draw_reward }
                    else {
                        (snake.is_alive() as u32 as f32) / (alive_count as f32)
                        * (snake.health as f32).powf(beta) / health_norm
                    };
    
                let rewards = board.snakes.iter().map(reward).collect();
    
                // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
                (simulation_id, rewards)
            })
        };
        
        self.simulation_pool.add_task(task);
    }

    fn backpropagate(&mut self, path: Vec<(u64, Vec<(usize, Action)>)>, rewards: Vec<f32>) {
        for (node_key, joint_action) in path {
            let node = self.nodes.get_mut(&node_key).unwrap();
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

    pub fn get_final_movement(&mut self, board: &Board, agent: usize) -> Movement {
        let node = self.nodes.get_mut(&board.zobrist_hash.get_value()).unwrap();
        let agent_index = node.get_agent_index(agent);
        let agent = &node.agents[agent_index];
        
        agent.bandit.get_final_movement(&self.config, node.visits)
    }

    pub fn shutdown(&self) {
        self.simulation_pool.shutdown();
    }
}
