use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::hash::BuildHasherDefault;
use std::thread;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use spin::mutex::Mutex;

use crate::api::objects::Movement;
use crate::engine::{EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::features::collector::Rewards;
use crate::game::{Board, MAX_SNAKE_COUNT};
use crate::mcts::heuristics::flood_fill::flavored_flood_fill;
use crate::mcts::search::Search;
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::{get_masks, get_random_actions_from_masks};

use super::wu_uct::WUUCT;
use super::config::ParallelMCTSConfig;

type Strategy = WUUCT;

pub struct Node {
    pub visits: f32,
    // Alive snakes
    pub agents: Vec<Agent>,
    pub unobserved_samples: f32,
}

pub struct Agent {
    id: usize,
    strategy: Strategy,
}


impl Node {
    fn new(agents: Vec<Agent>) -> Node {
        Node {
            agents,
            visits: 0.0,
            unobserved_samples: 0.0,
        }
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

impl Search for ParallelMCTS {
    fn search(&mut self, board: &Board, iterations_count: usize, verbose: bool) {
        let mut join_handles = Vec::with_capacity(self.config.workers);
        let iterations_per_worker = iterations_count / self.config.workers;
        
        for i in 0..self.config.workers {
            let mut worker = self.create_worker(i);
            let board_clone = board.clone();

            let join_handle = thread::spawn(move || {
                worker.search(board_clone, iterations_per_worker);
                worker.iterations
            });
            join_handles.push(join_handle);
        }

        let mut iterations = 0;
        join_handles.into_iter().for_each(|join_handle| {
            iterations += join_handle.join().unwrap();
        });

        if verbose {
            self.print_stats(board);
        }
    }

    fn search_with_time(&mut self, board: &Board, target_duration: Duration, verbose: bool) -> usize {
        let time_start = Instant::now();
        
        let mut join_handles = Vec::with_capacity(self.config.workers);
        for i in 0..self.config.workers {
            let mut worker = self.create_worker(i);
            let board_clone = board.clone();

            let join_handle = thread::spawn(move || {
                worker.search_with_time(board_clone, target_duration);
                worker.iterations
            });
            join_handles.push(join_handle);
        }

        let mut iterations = 0;
        join_handles.into_iter().for_each(|join_handle| {
            iterations += join_handle.join().unwrap();
        });

        if verbose {
            let actual_duration = Instant::now() - time_start;
            self.print_stats(board);
            println!("Searched {} iterations in {} ms (target={} ms)", iterations, actual_duration.as_millis(), target_duration.as_millis());
        }
        
        iterations
    }

    fn get_final_movement(&self, board: &Board, agent_index: usize, _verbose: bool) -> Movement {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock();
        let agent = &node.agents[agent_index];
        
        agent.strategy.get_final_movement()
    }

    fn shutdown(&self) {}
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

    #[allow(dead_code)]
    fn print_stats(&self, board: &Board) {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock();

        println!("Node visits: {}", node.visits);
        // info!("Max depth reached: {}", self.max_depth_reached.load(Ordering::));
        // let workers_wait_time = self.simulation_pool.workers_wait_time();
        // info!("Simulation workers wait time: {:?}; Worker avg wait: {}ms", workers_wait_time, workers_wait_time.as_millis() as f32 / self.config.simulation_workers as f32);
        // info!("Simulation result wait time: {:?}", self.simulation_pool.result_wait_time());
        // info!("Selection time: {:?}; avg: {}ms", self.selection_time, self.selection_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Expansion time: {:?}; avg: {}ms", self.expansion_time, self.expansion_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Backpropagate time: {:?}; avg: {}ms", self.backpropagate_time, self.backpropagate_time.as_millis() as f32 / self.iterations_complited as f32);
        // info!("Rollouts complited vs queued: {}/{}", self.iterations_complited, self.iteration);

        for agent in node.agents.iter() {
            println!("Snake {}", agent.id);
            agent.strategy.print_stats();
        }
    }

    
    fn create_worker(&self, id: usize) -> ParallelMCTSWorker {
        ParallelMCTSWorker::new(
            id,
            self.config.clone(),
            self.nodes.clone(),
            self.max_depth_reached.clone(),
        )
    }
}

struct ParallelMCTSWorker {
    id: usize,
    config: Arc<ParallelMCTSConfig>,
    nodes: Nodes,
    iterations: usize,
    max_depth_reached: Arc<AtomicUsize>,
}


impl ParallelMCTSWorker {
    pub fn new(id: usize, config: Arc<ParallelMCTSConfig>, nodes: Nodes, max_depth_reached: Arc<AtomicUsize>) -> ParallelMCTSWorker {
        ParallelMCTSWorker {
            id,
            nodes,
            config,
            max_depth_reached,
            iterations: 0,
        }
    }

    fn search_with_time(&mut self, board: Board, duration: Duration) {
        if self.nodes.len() == 0 {
            let masks = get_masks(&board);
            let node_key = board.zobrist_hash.get_value();
        
            self.expansion(&board, masks, node_key);
        }

        let time_start = Instant::now();
        let time_end = time_start + duration;

        while Instant::now() < time_end {
            self.rollout(&board);
            self.iterations += 1;
        }
    }

    pub fn search(&mut self, board: Board, iterations_count: usize) {
        if self.nodes.len() == 0 {
            let masks = get_masks(&board);
            let node_key = board.zobrist_hash.get_value();
        
            self.expansion(&board, masks, node_key);
        }

        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.rollout(&board);
            self.iterations += 1;
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

        let masks = get_masks(&board);
        let node_key = board.zobrist_hash.get_value();

        if !board.is_terminal() {
            self.expansion(&board, masks, node_key);
        }

        let rewards = self.simulation(&board);
        self.backpropagate(path, rewards);
    }

    fn selection(&self, board: &mut Board) -> Vec<(u64, [usize; MAX_SNAKE_COUNT])> {
        // let start = Instant::now();

        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::create_standard,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let iteration = self.iterations;

        while path.len() < self.config.max_select_depth {
            let node_key = board.zobrist_hash.get_value();
            let node_option = self.nodes.get(&node_key);

            if node_option.is_none() {
                break
            }
            let node_ref = node_option.unwrap();

            
            let mut joint_action = [0; MAX_SNAKE_COUNT];
            
            {
                let mut node = node_ref.lock();

                node.unobserved_samples += 1.0;
    
                let node_visits = node.visits;
                let node_unobserved_samples = node.unobserved_samples;

                let mut alive_i = 0;

                for snake_i in 0..board.snakes.len() {
                    if !board.snakes[snake_i].is_alive() {
                        continue;
                    }

                    let agent = &mut node.agents[alive_i];
                    let best_action = agent.strategy.get_best_movement(node_visits, node_unobserved_samples, iteration);
                    
                    agent.strategy.incomplete_update(best_action);
                    joint_action[snake_i] = best_action;                
    
                    alive_i += 1;
                }
            }
            
            advance_one_step_with_settings(
                board,
                &mut engine_settings,
                joint_action,
            );
            
            path.push((node_key, joint_action));
        }

        path
    }

    fn expansion(&self, board: &Board, masks: [[bool; 4]; MAX_SNAKE_COUNT], node_key: u64) {
        // let start = Instant::now();
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
        self.nodes.insert(node_key, Mutex::new(node));
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

    fn backpropagate(&self, path: Vec<(u64, [usize; MAX_SNAKE_COUNT])>, rewards: Rewards) {
        // let start = Instant::now();
        // info!("{:?}", self.nodes.keys());
        // info!("{:?}", path);
        for (node_key, joint_action) in path {
            let node_ref = self.nodes.get(&node_key).unwrap();
            let mut node = node_ref.lock();
            // info!("{}", node_key);
            node.visits += 1.0;
            node.unobserved_samples -= 1.0;

            for agent in &mut node.agents {
                let reward = rewards[agent.id];
                let movement = joint_action[agent.id];

                agent.strategy.backpropagate(reward, movement);
            }
        }
    }
}
