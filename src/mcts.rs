use rand::seq::SliceRandom;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use std::str::{FromStr};
use rand::distributions::WeightedIndex;
use rand::prelude::*;

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, MOVEMENTS, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Object, Point, Snake};

#[derive(Clone, Debug)]
struct RegretMatching {
    regret: [f32; 4],
    cumulative_p: [f32; 4],
    mask: [bool; 4],
}

impl RegretMatching {
    fn new(mask: [bool; 4]) -> RegretMatching {
        RegretMatching {
            regret: [0f32; 4],
            cumulative_p: [0f32; 4],
            mask: mask,
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    visits: u32,
    // Alive snakes
    agents: Vec<usize>,
    ucb_instances: Vec<RegretMatching>,
}

impl Node {
    fn new(agents: Vec<usize>, masks: Vec<[bool; 4]>) -> Node {
        Node {
            visits: 0,
            agents: agents,
            ucb_instances: masks.into_iter().map(RegretMatching::new).collect(),
        }
    }

    fn get_agent_index(&self, agent: usize) -> usize {
        self.agents.iter().copied().position(|a| a == agent).expect("There is no agent")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MCTSConfig {
    pub y: f32,
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
            y:              Self::parse_env("MCTS_Y").unwrap_or(0.05),
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
    config: MCTSConfig,
    nodes: HashMap<Board, RefCell<Node>>,
}

impl MCTS {
    pub fn new(config: MCTSConfig) -> MCTS {
        MCTS {
            config,
            nodes: HashMap::new(),
        }
    }

    pub fn print_stats(&self, board: &Board) {
        let node = self.nodes[board].borrow();

        for (ucb_instance, agent) in node.ucb_instances.iter().zip(node.agents.iter()) {
            info!("Snake {}", agent);
            let visits = node.visits as f32;
            // TODO: try max(0, pi / visits - 1 / moves_count)
            ucb_instance.cumulative_p
                .iter()
                .enumerate()
                .filter(|(a, _)| ucb_instance.mask[*a])
                .for_each(|(a, pi)| info!("{} {}", Movement::from_usize(a), *pi / visits));
        }
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.search_iteration(board);
        }
        // self.print_stats(board);
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
        // self.print_stats(board);
        info!("Searched {} iterations in {} ms (target={} ms)", i, actual_duration.as_millis(), target_duration.as_millis());
    }

    fn search_iteration(&mut self, board: &Board) {
        let mut board = board.clone();
        let mut path = Vec::new();
        let rng = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        while let Some(node_cell) = self.nodes.get(&board) {
            let mut node = node_cell.borrow_mut();

            let mut joint_probs = vec![0.0; node.ucb_instances.len()];

            let joint_action = advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let ucb_instance = &mut node.ucb_instances[agent_index];
                    let available_actions: Vec<_> = (0..4)
                        .filter(|&m| ucb_instance.mask[m])
                        .collect();
                    
                    if available_actions.is_empty() {
                        return Action::Move(Movement::Right);
                    }
                    
                    let positive_regret: Vec<_> = available_actions.iter()
                        .map(|&a| ucb_instance.regret[a].max(0.0))
                        .collect();
                    
                    let positive_regret_sum: f32 = positive_regret.iter().sum();
                    let mut p = if positive_regret_sum == 0.0 {
                        vec![1.0 / available_actions.len() as f32; available_actions.len()]
                    } else {
                        positive_regret.iter().map(|r| r/positive_regret_sum).collect()
                    };

                    for (pi, &a) in &mut p.iter_mut().zip(available_actions.iter()) {
                        *pi = (1.0 - self.config.y) * (*pi) + self.config.y / available_actions.len() as f32;
                        
                        ucb_instance.cumulative_p[a] += *pi;
                    }

                    let dist = WeightedIndex::new(&p).unwrap();
                    let best_index = dist.sample(rng);
                    joint_probs[agent_index] = p[best_index];
                    let best_action = available_actions[best_index];

                    Action::Move(Movement::from_usize(best_action))
                }
            );

            path.push((node, joint_action, joint_probs));
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
        let (alive_snakes, masks) = masks.into_iter().unzip();
        let node = Node::new(alive_snakes, masks);
        self.nodes.insert(board.clone(), RefCell::new(node));
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, Vec<(usize, Action)>, Vec<f32>)>, rewards: Vec<f32>) {
        for (mut node, joint_action, joint_probs) in path {
            node.visits += 1;

            for ((snake, Action::Move(movement)), p) in joint_action.into_iter().zip(joint_probs.into_iter()) {
                let snake_index = node.get_agent_index(snake);
                
                let reward = rewards[snake];
                let movement = movement as usize;

                let ucb = &mut node.ucb_instances[snake_index];
                for r in &mut ucb.regret {
                    *r -= reward;
                }

                let K = ucb.mask.iter().filter(|&&m| m).count() as f32;
                ucb.regret[movement] += reward / p;
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

        let reward = |snake: &Snake|
            if alive_count == 0 { self.config.draw_reward }
            else {
                (snake.is_alive() as u32 as f32) / (alive_count as f32)
                * (snake.health as f32).powf(beta) / health_norm
            } ;

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
        let ucb_instance = &node.ucb_instances[agent_index];
        let visits = node.visits as f32;
        // TODO: try max(0, pi / visits - 1 / moves_count)
        let (best_movement, _) = ucb_instance.cumulative_p
            .iter()
            .enumerate()
            .filter(|(a, _)| ucb_instance.mask[*a])
            .map(|(a, pi)| (a, *pi / visits))
            .max_by(|(_, a_normalized_pi), (_, b_normalized_pi),| a_normalized_pi.total_cmp(b_normalized_pi))
            .unwrap_or((0, 0.0));
    
        Movement::from_usize(best_movement)
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
                    let movement_position = get_movement_position(snake.body[0], movement);
                    if board.contains(movement_position)
                        && (tails.contains(&movement_position)
                            || board.objects[movement_position] != Object::BodyPart) {
                        movement_mask[movement as usize] = true;
                    }
                });
            (snake_id, movement_mask)
        })
        .collect()
}

fn get_movement_positions(position: Point) -> [Point; 4] {
    [
        get_movement_position(position, MOVEMENTS[0]),
        get_movement_position(position, MOVEMENTS[1]),
        get_movement_position(position, MOVEMENTS[2]),
        get_movement_position(position, MOVEMENTS[3]),
    ]
}

fn get_movement_position(position: Point, movement: Movement) -> Point {
    match movement {
        Movement::Right => Point {x: position.x + 1, y: position.y},
        Movement::Left => Point {x: position.x - 1, y: position.y},
        Movement::Up => Point {x: position.x, y: position.y + 1},
        Movement::Down => Point {x: position.x, y: position.y - 1},
    }
}
