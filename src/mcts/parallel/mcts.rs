use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::hash::BuildHasherDefault;
use std::thread;
use std::time::{Duration, Instant};

use rand::seq::SliceRandom;
use dashmap::DashMap;
use spin::mutex::Mutex;

use crate::api::objects::Movement;
use crate::engine::{EngineSettings, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, MAX_SNAKE_COUNT};
use crate::mcts::heuristics::flood_fill::flood_fill;
use crate::zobrist::ZobristHasher;
use crate::mcts::utils::get_masks;

use super::wu_uct::BUUCT;
use super::config::ParallelMCTSConfig;

type Strategy = BUUCT;

pub struct Node {
    pub visits: f32,
    // Alive snakes
    pub agents: Vec<Strategy>,
    pub unobserved_samples: f32,
}

impl Node {
    fn new(agents: Vec<Strategy>) -> Node {
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

impl Search<ParallelMCTSConfig> for ParallelMCTS {
    fn search(&mut self, board: &Board, iterations_count: u32) {
        let mut join_handles = Vec::with_capacity(self.config.workers);
        let iterations_per_worker = iterations_count / self.config.workers as u32;
        
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

        self.print_stats(board);
    }

    fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
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

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", iterations, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn get_final_movement(&self, board: &Board, agent_index: usize) -> Movement {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock();
        let agent = &node.agents[agent_index];
        
        agent.bandit.get_final_movement(&self.config, node.visits)
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

    fn print_stats(&self, board: &Board) {
        let node_ref = self.nodes.get(&board.zobrist_hash.get_value()).unwrap();
        let node = node_ref.lock();

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

    pub fn search(&mut self, board: Board, iterations_count: u32) {
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

                for agent_in_game_index in board.iter_alive_snakes() {
                    let agent_node_index = node.get_agent_index(agent_in_game_index);
                    let agent = &mut node.agents[agent_node_index];
                    let best_action = agent.bandit.get_best_movement(&self.config, node_visits, node_unobserved_samples, iteration);
                    
                    agent.bandit.incomplete_update(best_action);
                    joint_action[agent_in_game_index] = best_action;
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
        for snake_index in board.iter_alive_snakes() {
            agents.push(
                Agent {
                    id: snake_index,
                    bandit: Strategy::new(masks[snake_index])
                }
            );
        }
        
        let node = Node::new(agents);
        self.nodes.insert(node_key, Mutex::new(node));
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

            for snake_index in board.iter_alive_snakes() {
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
        
        let alive_count = board.iter_alive_snakes().count();
        if alive_count == 0 {
            return [self.config.draw_reward; MAX_SNAKE_COUNT];
        }

        let len_sum: f32 = board.snakes.iter().map(|snake| snake.body.len() as f32).sum();

        let mut rewards = flood_fill(&board);        
        
        for i in board.iter_alive_snakes() {
            rewards[i] *= board.snakes[i].body.len() as f32 / len_sum;
        }
        // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
        rewards
    }

    fn backpropagate(&self, path: Vec<(u64, [usize; MAX_SNAKE_COUNT])>, rewards: [f32; MAX_SNAKE_COUNT]) {
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

                agent.bandit.backpropagate(reward, movement);
            }
        }
    }
}
