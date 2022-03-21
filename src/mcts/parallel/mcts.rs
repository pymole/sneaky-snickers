use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::thread;
use std::time::{Duration, Instant};

use arrayvec::ArrayVec;
use rand::seq::SliceRandom;
use dashmap::DashMap;

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Snake, MAX_SNAKE_COUNT};
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


type Nodes = Arc<DashMap<u64, Mutex<Node>, BuildHasherDefault<ZobristHasher>>>;
pub struct ParallelMCTS {
    pub config: Arc<ParallelMCTSConfig>,
    nodes: Nodes,
    max_depth_reached: Arc<AtomicUsize>,

    // selection_time: Duration,
    // expansion_time: Duration,
    // backpropagate_time: Duration,
}

impl ParallelMCTS {
    pub fn new(config: ParallelMCTSConfig) -> ParallelMCTS {
        let nodes = Arc::new(DashMap::<u64, Mutex<Node>, BuildHasherDefault<ZobristHasher>>::with_capacity_and_hasher(config.table_capacity, BuildHasherDefault::<ZobristHasher>::default()));

        ParallelMCTS {
            config: Arc::new(config),
            nodes,
            max_depth_reached: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn print_stats(&self, board: &Board) {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock().unwrap();

        info!("Node visits: {}", node.visits);
        // info!("Max depth reached: {}", self.max_depth_reached.load(Ordering::));
        // let workers_wait_time = self.simulation_pool.workers_wait_time();
        // info!("Simulation workers wait time: {:?}; Worker avg wait: {}ms", workers_wait_time, workers_wait_time.as_millis() as f32 / self.config.simulation_workers as f32);
        // info!("Simulation result wait time: {:?}", self.simulation_pool.result_wait_time());
        // info!("Selection time: {:?}; avg: {}ms", self.selection_time, self.selection_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Expansion time: {:?}; avg: {}ms", self.expansion_time, self.expansion_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Backpropagate time: {:?}; avg: {}ms", self.backpropagate_time, self.backpropagate_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Rollouts complited vs queued: {}/{}", self.iterations_complited, self.iteration);

        for agent in node.agents.iter() {
            info!("Snake {}", agent.id);
            agent.bandit.print_stats(&self.config, node.visits);
        }
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        let mut join_handles = Vec::with_capacity(self.config.selection_workers);
        let iterations_per_worker = iterations_count / self.config.selection_workers as u32;
        
        for i in 0..self.config.selection_workers {
            let board_clone = board.clone();
            let config = self.config.clone();
            let nodes = self.nodes.clone();
            let max_depth_reached = self.max_depth_reached.clone();

            let join_handle = thread::spawn(move || {
                let mut worker = ParallelMCTSWorker::new(i, config, nodes, max_depth_reached);
                worker.search(board_clone, iterations_per_worker);
                worker.iterations_complited
            });
            join_handles.push(join_handle);
        }

        let mut iterations_complited = 0;

        join_handles.into_iter().for_each(|join_handle| {
            iterations_complited += join_handle.join().unwrap();
        });

        self.print_stats(board);
    }

    pub fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
        let time_start = Instant::now();
        
        let mut join_handles = Vec::with_capacity(self.config.selection_workers);
        for i in 0..self.config.selection_workers {
            let board_clone = board.clone();
            let config = self.config.clone();
            let nodes = self.nodes.clone();
            let max_depth_reached = self.max_depth_reached.clone();

            let join_handle = thread::spawn(move || {
                let mut worker = ParallelMCTSWorker::new(i, config, nodes, max_depth_reached);
                worker.search_with_time(board_clone, target_duration);
                worker.simulation_pool.shutdown();
                worker.iterations_complited
            });
            join_handles.push(join_handle);
        }

        let mut iterations_complited = 0;

        join_handles.into_iter().for_each(|join_handle| {
            iterations_complited += join_handle.join().unwrap();
        });

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", iterations_complited, actual_duration.as_millis(), target_duration.as_millis());
    }

    pub fn get_final_movement(&self, board: &Board, agent: usize) -> Movement {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock().unwrap();
        let agent_index = node.get_agent_index(agent);
        let agent = &node.agents[agent_index];
        
        agent.bandit.get_final_movement(&self.config, node.visits)
    }

    pub fn shutdown(&self) {
        
    }
}

struct ParallelMCTSWorker {
    id: usize,
    config: Arc<ParallelMCTSConfig>,
    nodes: Nodes,
    max_depth_reached: Arc<AtomicUsize>,
    simulation_pool: Pool<(usize, ArrayVec<f32, MAX_SNAKE_COUNT>)>,
    rollout_contexts: HashMap<usize, Vec<(u64, ArrayVec<(usize, Action), MAX_SNAKE_COUNT>)>>,
    
    current_iteration: usize,
    iterations_complited: usize,
}


impl ParallelMCTSWorker {
    pub fn new(id: usize, config: Arc<ParallelMCTSConfig>, nodes: Nodes, max_depth_reached: Arc<AtomicUsize>,
    ) -> ParallelMCTSWorker {
        ParallelMCTSWorker {
            id,
            nodes,
            simulation_pool: Pool::new(config.simulation_workers),
            config,
            max_depth_reached,
            rollout_contexts: HashMap::new(),
            current_iteration: 0,
            iterations_complited: 0,
        }
    }

    fn search_with_time(&mut self, board: Board, duration: Duration) {
        if self.nodes.len() == 0 {
            let masks = get_masks(&board);
            let node_key = board.zobrist_hash.get_value();
        
            self.expansion(masks, node_key);
        }

        let time_start = Instant::now();
        let time_end = time_start + duration;

        while Instant::now() < time_end {
            self.rollout(&board);
            self.current_iteration += 1;
        }
    }

    pub fn search(&mut self, board: Board, iterations_count: u32) {
        if self.nodes.len() == 0 {
            let masks = get_masks(&board);
            let node_key = board.zobrist_hash.get_value();
        
            self.expansion(masks, node_key);
        }

        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(&board);
            self.current_iteration += 1;
        }
    }

    fn rollout(&mut self, board: &Board) {
        // let start = Instant::now();
        let mut board = board.clone();

        let path = self.selection(&mut board);
        let depth = path.len();
        
        self.max_depth_reached.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |max_depth| {
            if max_depth < depth {
                Some(depth)
            } else {
                None
            }
        });

        self.rollout_contexts.insert(self.current_iteration, path);

        let masks = get_masks(&board);
        let node_key = board.zobrist_hash.get_value();

        if !board.is_terminal() {
            self.expansion(masks, node_key);
        }

        self.simulation(&board, self.current_iteration);

        if self.simulation_pool.tasks_count() < self.config.simulation_workers {
            return;
        }

        let (rollout_id, rewards) = self.simulation_pool.wait_result();
        let path = self.rollout_contexts.remove(&rollout_id).unwrap();

        self.backpropagate(path, rewards);
    }

    fn selection(&self, board: &mut Board) -> Vec<(u64, ArrayVec<(usize, Action), MAX_SNAKE_COUNT>)> {
        // let start = Instant::now();

        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let iteration = self.current_iteration;

        while path.len() < self.config.max_select_depth {
            let node_key = board.zobrist_hash.get_value();
            let node_option = self.nodes.get(&node_key);

            if node_option.is_none() {
                break
            }
            let node_ref = node_option.unwrap();

            let joint_action = {
                let mut node = node_ref.lock().unwrap();

                node.unobserved_samples += 1.0;
    
                let node_visits = node.visits;
                let node_unobserved_samples = node.unobserved_samples;
    
                // TODO: This is ugly. Maybe change snake_strategy on simple arguments pass.
                advance_one_step_with_settings(
                    board,
                    &mut engine_settings,
                    &mut |i, _| {
                        let agent_index = node.get_agent_index(i);
                        let agent = &mut node.agents[agent_index];
    
                        let best_action = agent.bandit.get_best_movement(&self.config, node_visits, node_unobserved_samples, iteration);
                        agent.bandit.incomplete_update(best_action);
                        Action::Move(Movement::from_usize(best_action))
                    }
                )
            };

            path.push((node_key, joint_action));
        }

        path
    }

    fn expansion(&self, masks: ArrayVec<(usize, [bool; 4]), MAX_SNAKE_COUNT>, node_key: u64) {
        // let start = Instant::now();

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

        self.nodes.insert(node_key, Mutex::new(node));
    }

    fn simulation(&mut self, board: &Board, rollout_id: usize) {
        let mut board = board.clone();
        let rollout_cutoff = self.config.rollout_cutoff;
        let draw_reward = self.config.draw_reward;

        let task = Task {
            fn_box: Box::new(move || {
                let random = &mut rand::thread_rng();
    
                let mut engine_settings = EngineSettings {
                    food_spawner: &mut food_spawner::create_standard,
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

    fn backpropagate(&mut self, path: Vec<(u64, ArrayVec<(usize, Action), MAX_SNAKE_COUNT>)>, rewards: ArrayVec<f32, MAX_SNAKE_COUNT>) {
        // let start = Instant::now();
        // info!("{:?}", self.nodes.keys());
        // info!("{:?}", path);
        for (node_key, joint_action) in path.into_iter().rev() {
            let node_ref = self.nodes.get(&node_key).unwrap();
            let mut node = node_ref.lock().unwrap();
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
    }
}
