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

trait Formula {
    fn get_best_movement(&mut self, mcts_config: &MCTSConfig, node_visits: u32) -> (usize, ActionContext);
    fn get_final_movement(&self, mcts_config: &MCTSConfig, node_visits: u32) -> Movement;
    fn backpropagate(&mut self, reward: f32, movement: usize, action_context: ActionContext);
    fn print_stats(&self, mcts_config: &MCTSConfig, node_visits: u32);
}

#[derive(Clone, Debug)]
struct UCB {
    rewards: [f32; 4],
    squared_rewards: [f32; 4],
    visits: [u32; 4],
    mask: [bool; 4],
}

impl UCB {
    fn new(mask: [bool; 4]) -> UCB {
        UCB {
            rewards: [0f32; 4],
            squared_rewards: [0f32; 4],
            visits: [0u32; 4],
            mask: mask,
        }
    }
}

impl Formula for UCB {
    fn get_best_movement(&mut self, _mcts_config: &MCTSConfig, node_visits: u32) -> (usize, ActionContext) {
        let mut max_ucb_action = 0;
        let mut max_ucb = -1.0;

        for action in (0..4).filter(|&m| self.mask[m]) {
            let n = node_visits as f32;
            let n_i = self.visits[action] as f32;

            let ucb = if n_i > 0.0 {
                let avg_reward = self.rewards[action] / n_i;
                let avg_squared_reward = self.squared_rewards[action] / n_i;
                let variance = avg_squared_reward - avg_reward * avg_reward;
                let variance_ucb = (variance + (2.0 * n.ln() / n_i).sqrt()).min(0.25);

                avg_reward + (variance_ucb * n.ln() / n_i).sqrt()
            } else {
                f32::INFINITY
            };

            if ucb > max_ucb {
                max_ucb_action = action;
                max_ucb = ucb;
            }
        }

        (max_ucb_action, ActionContext::UCB)
    }

    fn get_final_movement(&self, _mcts_config: &MCTSConfig, _node_visits: u32) -> Movement {
        let (best_movement, _) = self.visits
            .iter()
            .enumerate()
            .max_by_key(|(_, &visits)| visits)
            .unwrap_or((0, &0));

        Movement::from_usize(best_movement)
    }

    fn backpropagate(&mut self, reward: f32, movement: usize, _action_context: ActionContext) {
        self.rewards[movement] += reward;
        self.squared_rewards[movement] += reward * reward;
        self.visits[movement] += 1;
    }

    fn print_stats(&self, _mcts_config: &MCTSConfig, node_visits: u32) {
        let n = node_visits as f32;

        info!("UCB   reward  explore  visits");

        let selected_move = self.get_final_movement(_mcts_config, node_visits);
        for action in 0..4 {
            let n_i = self.visits[action] as f32;
            if n_i > 0.0 {
                // copy-paste
                let avg_reward = self.rewards[action] / n_i;
                let avg_squared_reward = self.squared_rewards[action] / n_i;
                let variance = avg_squared_reward - avg_reward * avg_reward;
                let variance_ucb = (variance + (2.0 * n.ln() / n_i).sqrt()).min(0.25);

                let explore = (variance_ucb * n.ln() / n_i).sqrt();

                if action == selected_move as usize {
                    info!(
                        "[{}] - {:.4}  {:.4}   {}",
                        Movement::from_usize(action),
                        avg_reward,
                        explore,
                        n_i
                    );
                } else {
                    info!(
                        " {}  - {:.4}  {:.4}   {}",
                        Movement::from_usize(action),
                        avg_reward,
                        explore,
                        n_i
                    );
                }
            } else {
                info!(" {}", Movement::from_usize(action));
            }
        }
    }
}

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

impl Formula for RegretMatching {

    fn get_best_movement(&mut self, config: &MCTSConfig, _node_visits: u32) -> (usize, ActionContext) {
        let rng = &mut rand::thread_rng();
        let available_actions: Vec<_> = (0..4)
            .filter(|&m| self.mask[m])
            .collect();

        if available_actions.is_empty() {
            return (0, ActionContext::RM(1.0 / available_actions.len() as f32));
        }

        let positive_regret: Vec<_> = available_actions.iter()
            .map(|&a| self.regret[a].max(0.0))
            .collect();

        let positive_regret_sum: f32 = positive_regret.iter().sum();
        let mut p = if positive_regret_sum == 0.0 {
            vec![1.0 / available_actions.len() as f32; available_actions.len()]
        } else {
            positive_regret.iter().map(|r| r/positive_regret_sum).collect()
        };

        for (pi, &a) in &mut p.iter_mut().zip(available_actions.iter()) {
            *pi = (1.0 - config.y) * (*pi) + config.y / available_actions.len() as f32;

            self.cumulative_p[a] += *pi;
        }

        let dist = WeightedIndex::new(&p).unwrap();
        let best_index = dist.sample(rng);
        let best_action = available_actions[best_index];

        (best_action, ActionContext::RM(p[best_index]))
    }

    fn get_final_movement(&self, _mcts_config: &MCTSConfig, node_visits: u32) -> Movement {
        let n = node_visits as f32;
        // TODO: try max(0, pi / visits - 1 / moves_count)
        let (best_movement, _) = self.cumulative_p
            .iter()
            .enumerate()
            .filter(|(a, _)| self.mask[*a])
            .map(|(a, pi)| (a, *pi / n))
            .max_by(|(_, a_normalized_pi), (_, b_normalized_pi),| a_normalized_pi.total_cmp(b_normalized_pi))
            .unwrap_or((0, 0.0));

        Movement::from_usize(best_movement)
    }

    fn backpropagate(&mut self, reward: f32, movement: usize, action_context: ActionContext) {
        match action_context {
            ActionContext::RM(p) => {
                for r in &mut self.regret {
                    *r -= reward;
                }

                // let k = self.mask.iter().filter(|&&m| m).count() as f32;
                self.regret[movement] += reward / p;
            },
            _ => panic!("Wrong context"),
        }
    }

    fn print_stats(&self, _mcts_config: &MCTSConfig, node_visits: u32) {
        info!("RM");
        let visits = node_visits as f32;
        self.cumulative_p
            .iter()
            .enumerate()
            .filter(|(a, _)| self.mask[*a])
            .for_each(|(a, pi)| info!("{} {}", Movement::from_usize(a), *pi / visits));
    }
}

#[derive(Clone)]
enum ActionContext {
    UCB,
    RM(f32),
}

struct Node {
    visits: u32,
    // Alive snakes
    agents: Vec<Agent>,
}

struct Agent {
    id: usize,
    data: Box<dyn Formula + Send>,
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
            iterations:     Self::parse_env("MCTS_ITERATIONS"),
            search_time:    Self::parse_env("MCTS_SEARCH_TIME").map(Duration::from_millis),
            rollout_cutoff: Self::parse_env("MCTS_ROLLOUT_CUTOFF").unwrap_or(21),
            rollout_beta:   Self::parse_env("MCTS_ROLLOUT_BETA").unwrap_or(3.0),
            draw_reward:    Self::parse_env("MCTS_DRAW_REWARD").unwrap_or(NORMALIZED_DRAW_REWARD),
        };

        assert!(config.rollout_cutoff >= 0);

        config
    }
}

pub struct MCTS {
    pub config: MCTSConfig,
    // TODO: maybe put this nodes in one table
    nodes: HashMap<Board, RefCell<Node>>,
}

impl MCTS {
    pub fn new(config: MCTSConfig) -> MCTS {
        MCTS {
            config,
            nodes: HashMap::new(),
        }
    }

    fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(board).unwrap().borrow();

        for agent in node.agents.iter() {
            info!("Snake {}", agent.id);
            agent.data.print_stats(&self.config, node.visits);
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
            let mut action_contexts = vec![ActionContext::UCB; node.agents.len()];

            let joint_action = advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let agent = &mut node.agents[agent_index];

                    let (best_action, context) = agent.data.get_best_movement(&self.config, n);
                    action_contexts[agent_index] = context;
                    Action::Move(Movement::from_usize(best_action))
                }
            );

            path.push((node, joint_action, action_contexts));
        }

        let expansion_board = board.clone();
        // let start_rollout = Instant::now() - start;
        let rewards = self.rollout(&mut board);
        // let start_backpropagate = Instant::now() - start;
        self.backpropagate(path, rewards);
        // let start_exapantion = Instant::now() - start;
        if !expansion_board.is_terminal() {
            self.expansion(&expansion_board);
        }
    }

    fn expansion(&mut self, board: &Board) {
        // let start = Instant::now();
        let masks = get_masks_with_risk_flag(board);
        // let mask_duration = Instant::now() - start;
        let agents = masks
            .into_iter()
            .map(|(agent, mask, risk)| {
                if risk {
                    Agent {
                        id: agent,
                        data: Box::new(RegretMatching::new(mask))
                    }
                } else {
                    Agent {
                        id: agent,
                        data: Box::new(UCB::new(mask))
                    }
                }
            })
            .collect();
        // let agent_duration = Instant::now() - start;

        let node = Node::new(agents);
        // let node_create = Instant::now() - start;

        let board_clone = board.clone();
        // let board_clone_time = Instant::now() - start;

        // let c = self.nodes.capacity();
        self.nodes.insert(board_clone, RefCell::new(node));
        // let c_new = self.nodes.capacity();

        // let end = Instant::now() - start;
        // if end > Duration::from_millis(10) {
        //     info!("EX {:?} {:?} {:?} {:?} {:?} {} {} {}", mask_duration, agent_duration, node_create, board_clone_time, end, self.nodes.len(), c, c_new);
        // }
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, Vec<(usize, Action)>, Vec<ActionContext>)>, rewards: Vec<f32>) {
        for (mut node, joint_action, action_contexts) in path {
            node.visits += 1;

            for ((agent, Action::Move(movement)), action_context) in joint_action.into_iter().zip(action_contexts.into_iter()) {
                let agent_index = node.get_agent_index(agent);

                let reward = rewards[agent];
                let movement = movement as usize;

                let agent = &mut node.agents[agent_index];
                agent.data.backpropagate(reward, movement, action_context);
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
        let agent = &node.agents[agent_index];
        agent.data.get_final_movement(&self.config, node.visits)
    }
}

fn get_masks_with_risk_flag(board: &Board) -> Vec<(usize, [bool; 4], bool)> {
    let tails: Vec<_> = board.snakes
        .iter()
        .filter(|snake| snake.is_alive())
        .map(|snake| *snake.body.back().unwrap())
        .collect();

    let mut all_movement_positions = HashMap::new();

    let masks: Vec<_> = board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .map(|(snake_id, snake)| {
            let mut movement_mask = [false; 4];
            let movement_positions = get_movement_positions(snake.body[0]);

            for (&movement_position, &movement) in movement_positions.iter().zip(MOVEMENTS.iter()) {
                if board.contains(movement_position) && (tails.contains(&movement_position)
                        || board.objects[movement_position] != Object::BodyPart) {

                    movement_mask[movement as usize] = true;

                    if let Some(multiple_snakes) = all_movement_positions.get_mut(&movement_position) {
                        *multiple_snakes = true;
                    } else {
                        all_movement_positions.insert(movement_position, false);
                    }
                }
            }

            (snake_id, movement_mask, movement_positions)
        })
        .collect();

    // info!("{:?}", movement_position_max_body);
    masks.into_iter()
        .map(|(snake_id, movement_mask, movement_positions)| {
            let mut risk = false;
            for (mask, position) in movement_mask.iter().zip(movement_positions.iter()) {
                if *mask && *all_movement_positions.get(position).unwrap() {
                    risk = true;
                    break;
                }
            }
            (snake_id, movement_mask, risk)
        })
        .collect()
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
