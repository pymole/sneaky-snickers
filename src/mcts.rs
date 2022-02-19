use rand::seq::SliceRandom;
use rand::prelude::*;
use rand::thread_rng;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use std::str::FromStr;

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, MOVEMENTS, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Object, Point, Snake};
use crate::bandit::MultiArmedBandit;


// TODO: cfg for ucb and thompson
// #[cfg(strategy = "ucb")]
// use crate::ucb::UCB;
// #[cfg(strategy = "ucb")]
// type Strategy = UCB;

// #[cfg(strategy = "thompson")]
use crate::thompson::ThompsonSampling;
// #[cfg(strategy = "thompson")]
type Strategy = ThompsonSampling;


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

#[derive(Clone, Copy, Debug)]
pub struct MCTSConfig {
    pub y: f32,
    pub table_capacity: usize,
    pub iterations: Option<u32>,
    pub search_time: Option<Duration>,
    pub rollout_cutoff: i32,
    pub rollout_beta: f32,
    pub draw_reward: f32,
}

impl MCTSConfig {
    fn parse_env<Value>(key: &str) -> Option<Value>
    where
        Value: FromStr,
        <Value as FromStr>::Err: std::fmt::Debug
    {
        env::var(key).map(|s| s.parse().expect("MCTSConfig: can't parse env variable")).ok()
    }

    pub fn from_env() -> MCTSConfig {
        const WIN_POINTS : f32 = 15.0;
        const DRAW_POINTS : f32 = 0.0;
        const LOSS_POINTS : f32 = -3.0;
        const NORMALIZED_DRAW_REWARD : f32 = (DRAW_POINTS - LOSS_POINTS) / (WIN_POINTS - LOSS_POINTS);

        let config = MCTSConfig {
            y:              Self::parse_env("MCTS_Y").unwrap_or(0.8),
            table_capacity: Self::parse_env("MCTS_TABLE_CAPACITY").unwrap_or(200000),
            iterations:     Self::parse_env("MCTS_ITERATIONS"),
            search_time:    Self::parse_env("MCTS_SEARCH_TIME").map(Duration::from_millis),
            rollout_cutoff: Self::parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(21),
            rollout_beta:   Self::parse_env("MCTS_ROLLOUT_BETA").unwrap_or(3.0),
            draw_reward:    Self::parse_env("MCTS_DRAW_REWARD").unwrap_or(NORMALIZED_DRAW_REWARD),
        };

        assert!(config.rollout_cutoff >= 1);

        config
    }
}

pub struct MCTS {
    pub config: MCTSConfig,
    nodes: HashMap<Board, RefCell<Node>>,
}

impl MCTS {
    pub fn new(config: MCTSConfig) -> MCTS {
        MCTS {
            nodes: HashMap::with_capacity(config.table_capacity),
            config,
        }
    }

    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(board).unwrap().borrow();

        for agent in node.agents.iter() {
            info!("Snake {}", agent.id);
            agent.bandit.print_stats(&self.config, node.visits);
        }
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.search_iteration(board);
        }
        self.print_stats(board);
    }

    pub fn search_with_time(&mut self, board: &Board, target_duration: Duration) {
        let time_start = Instant::now();
        let time_end = time_start + target_duration;

        let mut i = 0u32;
        while Instant::now() < time_end {
            self.search_iteration(board);
            i += 1;
        }

        let actual_duration = Instant::now() - time_start;
        self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", i, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn search_iteration(&mut self, board: &Board) {
        // let start = Instant::now();
        let mut board = board.clone();
        let mut path = Vec::new();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        while let Some(node_cell) = self.nodes.get(&board) {
            let mut node = node_cell.borrow_mut();
            let n = node.visits;

            // TODO: This is ugly. Maybe change snake_strategy on simple arguments pass.
            let joint_action = advance_one_step_with_settings(
                &mut board,
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

        let expansion_board = board.clone();
        let rewards = self.rollout(&mut board);
        self.backpropagate(path, rewards);
        if !expansion_board.is_terminal() {
            self.expansion(&expansion_board);
        }
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
        let board_clone = board.clone();
        self.nodes.insert(board_clone, RefCell::new(node));
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, Vec<(usize, Action)>)>, rewards: Vec<f32>) {
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

    fn rollout(&self, board: &mut Board) -> Vec<f32> {
        let random = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        let end_turn = board.turn + self.config.rollout_cutoff;
        while board.turn <= end_turn && !board.is_terminal() {
            let actions: HashMap<_, _> = get_masks(board)
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
                board,
                &mut engine_settings,
                &mut |snake, _| *actions.get(&snake).unwrap()
            );
        }
        
        let alive_count = board.snakes.iter().filter(|snake| snake.is_alive()).count();
        let beta = self.config.rollout_beta;
        let health_norm : f32 = board.snakes.iter().map(|snake| (snake.health as f32).powf(beta)).sum();
        let reward = |snake: &Snake| {
            let r = if alive_count == 0 {
                self.config.draw_reward
            }
            else {
                (snake.is_alive() as u32 as f32) / (alive_count as f32)
                * (snake.health as f32).powf(beta) / health_norm
            };
            let sampled_r: f32 = rand::random();
            if sampled_r < r { 1.0 } else { 0.0 }
        };

        let rewards = board.snakes.iter().map(reward).collect();
        

        // info!("Started at {} turn and rolled out with {} turns and rewards {:?}", start_turn, board.turn - start_turn, rewards);
        rewards
    }
}

pub trait GetBestMovement {
    fn get_best_movement(&self, board: &Board, agent: usize) -> Movement;
}

impl GetBestMovement for MCTS {
    fn get_best_movement(&self, board: &Board, agent: usize) -> Movement {
        let node = self.nodes[board].borrow();
        let agent_index = node.get_agent_index(agent);
        let agent = &node.agents[agent_index];
        agent.bandit.get_final_movement(&self.config, node.visits)
    }
}

fn get_masks(board: &Board) -> Vec<(usize, [bool; 4])> {
    let tails: Vec<_> = board.snakes
        .iter()
        .filter(|snake| snake.is_alive())
        .map(|snake| *snake.body.back().unwrap())
        .collect();

    board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .map(|(snake_id, snake)| {
            let mut movement_mask = [false; 4];
            MOVEMENTS
                .iter()
                .for_each(|&movement| {
                    let movement_position = get_movement_position(snake.body[0], movement, board.size);
                    if tails.contains(&movement_position) || board.objects[movement_position] != Object::BodyPart {
                        movement_mask[movement as usize] = true;
                    }
                });
            (snake_id, movement_mask)
        })
        .collect()
}

// fn get_movement_positions(position: Point) -> [Point; 4] {
//     [
//         get_movement_position(position, MOVEMENTS[0]),
//         get_movement_position(position, MOVEMENTS[1]),
//         get_movement_position(position, MOVEMENTS[2]),
//         get_movement_position(position, MOVEMENTS[3]),
//     ]
// }

fn get_movement_position(position: Point, movement: Movement, borders: Point) -> Point {
    match movement {
        Movement::Right => Point {x: (position.x + 1) % borders.x, y: position.y},
        Movement::Left => Point {x: (borders.x + position.x - 1) % borders.x, y: position.y},
        Movement::Up => Point {x: position.x, y: (position.y + 1) % borders.y },
        Movement::Down => Point {x: position.x, y: (borders.y + position.y - 1) % borders.y},
    }
}
