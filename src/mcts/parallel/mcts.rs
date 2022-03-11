use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::{Duration, Instant};
use rand::seq::SliceRandom;

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Snake};
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::get_masks;

use super::bu_uct::BUUCT;
use super::config::ParallelMCTSConfig;
use super::pool::{Pool, Task};

type Strategy = BUUCT;

pub struct Node {
    pub visits: f32,
    // Alive snakes
    pub agents: Vec<Agent>,
    pub unobserved_samples: f32,
}

pub struct Agent {
    id: usize,
    bandit: Strategy,
}

impl Node {
    fn new(agents: Vec<Agent>) -> Node {
        Node {
            agents,
            visits: 0.0,
            unobserved_samples: 0.0,
        }
    }

    fn get_agent_index(&self, agent_id: usize) -> usize {
        self.agents.iter().position(|a| a.id == agent_id).expect("There is no agent")
    }
}

pub struct ParallelMCTS {
    pub config: ParallelMCTSConfig,
    nodes: HashMap<u64, Arc<Mutex<Node>>, BuildHasherDefault<ZobristHasher>>,
    simulation_pool: Pool<(usize, Vec<f32>)>,
    iteration: usize,
    iterations_complited: usize,
    max_depth_reached: usize,

    selection_time: Duration,
    expansion_time: Duration,
    backpropagate_time: Duration,
}


impl ParallelMCTS {
    pub fn new(config: ParallelMCTSConfig) -> ParallelMCTS {
        let simulation_pool = Pool::<(usize, Vec<f32>)>::new(config.simulation_workers);

        ParallelMCTS {
            nodes: HashMap::with_capacity_and_hasher(config.table_capacity, BuildHasherDefault::<ZobristHasher>::default()),
            config,
            simulation_pool,
            iteration: 0,
            iterations_complited: 0,
            max_depth_reached: 0,
            selection_time: Duration::new(0, 0),
            expansion_time: Duration::new(0, 0),
            backpropagate_time: Duration::new(0, 0),
        }
    }

    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(&board.zobrist_hash.get_value()).unwrap().lock().unwrap();

        info!("Node visits: {}", node.visits);
        info!("Max depth reached: {}", self.max_depth_reached);
        info!("Simulation pool wait time: {:?}; Worker avg wait: {}ms", self.simulation_pool.wait_time(), self.simulation_pool.wait_time().as_millis() as f32 / self.config.simulation_workers as f32);
        info!("Selection time: {:?}; avg: {}ms", self.selection_time, self.selection_time.as_millis() as f32 / self.iterations_complited as f32);
        info!("Expansion time: {:?}; avg: {}ms", self.expansion_time, self.expansion_time.as_millis() as f32 / self.iterations_complited as f32);
        info!("Backpropagate time: {:?}; avg: {}ms", self.backpropagate_time, self.backpropagate_time.as_millis() as f32 / self.iterations_complited as f32);
        info!("Rollouts complited vs queued: {}/{}", self.iterations_complited, self.iteration);

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
            self.iteration += 1;
        }
        self.print_stats(board);
    }

    pub fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
        let mut rollout_contexts = HashMap::with_capacity(self.config.simulation_workers);

        let time_start = Instant::now();
        let time_end = time_start + target_duration;

        while Instant::now() < time_end {
            self.rollout(board, &mut rollout_contexts);
            self.iteration += 1;
        }

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", self.iterations_complited, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn rollout(&mut self, board: &Board, rollout_contexts: &mut HashMap<usize, Vec<(Arc<Mutex<Node>>, Vec<(usize, Action)>)>>) {
        // let start = Instant::now();
        let mut board = board.clone();
        
        if self.nodes.len() == 0 {
            self.expansion(&board);
        }

        let path = self.selection(&mut board);

        if self.max_depth_reached < path.len() {
            self.max_depth_reached = path.len();
        }
        rollout_contexts.insert(self.iteration, path);

        if !board.is_terminal() {
            self.expansion(&board);
        }

        self.simulation(&board, self.iteration);

        if self.simulation_pool.tasks_count() < self.config.simulation_workers {
            return;
        }

        let (rollout_id, rewards) = self.simulation_pool.wait_result();
        // info!("Sims {:?}", simulation_contexts.keys());
        // info!("Sim id {}", simulation_id);
        let path = rollout_contexts.remove(&rollout_id).unwrap();

        self.backpropagate(path, rewards);
    }

    fn selection(&mut self, board: &mut Board) -> Vec<(Arc<Mutex<Node>>, Vec<(usize, Action)>)> {
        let start = Instant::now();

        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let config = &self.config;
        let iteration = self.iteration;

        while path.len() < self.config.max_select_depth {
            let node_option = self.nodes.get(&board.zobrist_hash.get_value());
            if node_option.is_none() {
                break
            }
            let node_arc = node_option.unwrap();
            let mut node = node_arc.lock().unwrap();

            node.unobserved_samples += 1.0;

            let node_visits = node.visits;
            let node_unobserved_samples = node.unobserved_samples;

            // TODO: This is ugly. Maybe change snake_strategy on simple arguments pass.
            let joint_action = advance_one_step_with_settings(
                board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let agent = &mut node.agents[agent_index];

                    let best_action = agent.bandit.get_best_movement(config, node_visits, node_unobserved_samples, iteration);
                    agent.bandit.incomplete_update(best_action);
                    Action::Move(Movement::from_usize(best_action))
                }
            );

            path.push((Arc::clone(node_arc), joint_action));
        }

        self.selection_time += Instant::now() - start;

        path
    }

    fn expansion(&mut self, board: &Board) {
        let start = Instant::now();

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
        self.nodes.insert(board.zobrist_hash.get_value(), Arc::new(Mutex::new(node)));

        self.expansion_time += Instant::now() - start;
    }

    fn simulation(&mut self, board: &Board, rollout_id: usize) {
        let mut board = board.clone();
        let rollout_cutoff = self.config.rollout_cutoff;
        let draw_reward = self.config.draw_reward;

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
                let len_norm: f32 = board.snakes.iter().map(|snake| snake.body.len() as f32).sum();

                let reward = |snake: &Snake|
                    if alive_count == 0 { draw_reward }
                    else {
                        (snake.is_alive() as u32 as f32) / (alive_count as f32)
                        * (snake.body.len() as f32) / len_norm
                    } ;

                let rewards = board.snakes.iter().map(reward).collect();

                // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
                (rollout_id, rewards)
            })
        };
        
        self.simulation_pool.add_task(task);
    }

    fn backpropagate(&mut self, path: Vec<(Arc<Mutex<Node>>, Vec<(usize, Action)>)>, rewards: Vec<f32>) {
        let start = Instant::now();
        // info!("{:?}", self.nodes.keys());
        // info!("{:?}", path);
        for (node_arc, joint_action) in path {
            let mut node = node_arc.lock().unwrap();
            // info!("{}", node_key);
            node.visits += 1.0;
            node.unobserved_samples -= 1.0;

            for (agent, Action::Move(movement)) in joint_action.into_iter() {
                let agent_index = node.get_agent_index(agent);

                let reward = rewards[agent];
                let movement = movement as usize;

                let agent = &mut node.agents[agent_index];
                agent.bandit.backpropagate(reward, movement);
            }
        }
        self.iterations_complited += 1;

        self.backpropagate_time += Instant::now() - start;
    }

    pub fn get_final_movement(&mut self, board: &Board, agent: usize) -> Movement {
        let node = self.nodes.get(&board.zobrist_hash.get_value()).unwrap().lock().unwrap();
        let agent_index = node.get_agent_index(agent);
        let agent = &node.agents[agent_index];
        
        agent.bandit.get_final_movement(&self.config, node.visits)
    }

    pub fn shutdown(&self) {
        self.simulation_pool.shutdown();
    }
}
