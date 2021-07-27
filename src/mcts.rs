use rand::seq::SliceRandom;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use std::str::{FromStr};

use crate::api::objects::Movement;
use crate::engine::{Action, EngineSettings, MOVEMENTS, advance_one_step_with_settings, food_spawner, safe_zone_shrinker};
use crate::game::{Board, Object, Point, Snake};

#[derive(Clone, Debug)]
struct UCB {
    rewards: [f32; 4],
    squared_rewards: [f32; 4],
    visits: [u32; 4],
    mask: [MovementMask; 4],
}

impl UCB {
    fn new(mask: [MovementMask; 4]) -> UCB {
        UCB {
            rewards: [0f32; 4],
            squared_rewards: [0f32; 4],
            visits: [0u32; 4],
            mask: mask,
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    visits: u32,
    // Alive snakes
    agents: Vec<usize>,
    ucb_instances: Vec<UCB>,
}

impl Node {
    fn new(agents: Vec<usize>, masks: Vec<[MovementMask; 4]>) -> Node {
        Node {
            visits: 0,
            agents: agents,
            ucb_instances: masks.into_iter().map(UCB::new).collect(),
        }
    }

    fn get_agent_index(&self, agent: usize) -> usize {
        self.agents.iter().copied().position(|a| a == agent).expect("There is no agent")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MCTSConfig {
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

    pub fn get_movement_visits(&self, board: &Board, snake: usize) -> [u32; 4] {
        let node = self.nodes.get(board).unwrap().borrow();
        let snake_index = node.get_agent_index(snake);
        let ucb_instance = &node.ucb_instances[snake_index];

        ucb_instance.visits
    }

    pub fn print_stats(&self, board: &Board) {
        let node = self.nodes.get(board).unwrap().borrow();
        let n = node.visits as f32;

        info!("      reward  explore  visits");

        for (&snake_index, ucb_instance) in node.agents.iter().zip(node.ucb_instances.iter()) {
            info!("Snake {}", snake_index);
            let selecte_move = get_best_movement_from_movement_visits(ucb_instance.visits);
            for action in 0..4 {
                let n_i = ucb_instance.visits[action] as f32;
                if n_i > 0.0 {
                    // copy-paste
                    let avg_reward = ucb_instance.rewards[action] / n_i;
                    let avg_squared_reward = ucb_instance.squared_rewards[action] / n_i;
                    let variance = avg_squared_reward - avg_reward * avg_reward;
                    let variance_ucb = (variance + (2.0 * n.ln() / n_i).sqrt()).min(0.25);

                    let explore = (variance_ucb * n.ln() / n_i).sqrt();

                    if action == selecte_move {
                        info!(
                            "[{}] - {:.4}  {:.4}   {}",
                            Movement::from_usize(action),
                            avg_reward,
                            explore,
                            n_i,
                        );
                    } else {
                        info!(
                            " {}  - {:.4}  {:.4}   {}",
                            Movement::from_usize(action),
                            avg_reward,
                            explore,
                            n_i,
                        );
                    }
                } else {
                    info!(" {}", Movement::from_usize(action));
                }
            }
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
        let mut board = board.clone();
        let mut path = Vec::new();
        let rng = &mut rand::thread_rng();

        let mut engine_settings = EngineSettings {
            food_spawner: &mut food_spawner::noop,
            safe_zone_shrinker: &mut safe_zone_shrinker::standard,
        };

        while let Some(node_cell) = self.nodes.get(&board) {
            let node = node_cell.borrow_mut();

            let joint_action = advance_one_step_with_settings(
                &mut board,
                &mut engine_settings,
                &mut |i, _| {
                    let agent_index = node.get_agent_index(i);
                    let ucb_instance = &node.ucb_instances[agent_index];
                    let (safe_actions, risky_actions): (Vec<_>, Vec<_>) = (0..4)
                        .filter(|&m| ucb_instance.mask[m] != MovementMask::Forbidden)
                        .partition(|&m| ucb_instance.mask[m] == MovementMask::Safe);
                
                    let best_action = if !safe_actions.is_empty() {
                        let mut max_ucb_action = 0;
                        let mut max_ucb = -1.0;

                        for action in safe_actions {
                            let n = node.visits as f32;
                            let n_i = ucb_instance.visits[action] as f32;

                            let ucb = if n_i > 0.0 {
                                let avg_reward = ucb_instance.rewards[action] / n_i;
                                let avg_squared_reward = ucb_instance.squared_rewards[action] / n_i;
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
                        max_ucb_action
                    } else {
                        // TODO: proportional sampling
                        *risky_actions.choose(rng).unwrap_or(&0)
                    };
                    

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
        let (alive_snakes, masks) = masks.into_iter().unzip();
        let node = Node::new(alive_snakes, masks);
        self.nodes.insert(board.clone(), RefCell::new(node));
    }

    fn backpropagate(&self, path: Vec<(RefMut<Node>, Vec<(usize, Action)>)>, rewards: Vec<f32>) {
        for (mut node, joint_action) in path {
            node.visits += 1;

            for (snake, Action::Move(movement)) in joint_action {
                let snake_index = node.get_agent_index(snake);
                
                let reward = rewards[snake];
                let movement = movement as usize;

                let ucb = &mut node.ucb_instances[snake_index];
                ucb.rewards[movement] += reward;
                ucb.squared_rewards[movement] += reward * reward;
                ucb.visits[movement] += 1;
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
                        .filter(|(_, &mask)| mask != MovementMask::Forbidden)
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum MovementMask {
    Risky,
    Safe,
    Forbidden,
}

fn get_masks(board: &Board) -> Vec<(usize, [MovementMask; 4])> {
    let tails: Vec<_> = board.snakes
        .iter()
        .filter(|snake| snake.is_alive())
        .map(|snake| *snake.body.back().unwrap())
        .collect();
    
    let mut movement_position_max_body = HashMap::new();

    let masks: Vec<_> = board.snakes
        .iter()
        .enumerate()
        .filter(|(_, snake)| snake.is_alive())
        .map(|(snake_id, snake)| {
            let mut movement_mask = [MovementMask::Forbidden; 4];
            let movement_positions = get_movement_positions(snake.body[0]);
            let snake_length = snake.body.len();

            for (&movement_position, &movement) in movement_positions.iter().zip(MOVEMENTS.iter()) {
                if board.contains(movement_position) && (tails.contains(&movement_position)
                        || board.objects[movement_position] != Object::BodyPart) {
                    
                    movement_mask[movement as usize] = MovementMask::Safe;
                    
                    if let Some(&(_, max_body)) = movement_position_max_body.get(&movement_position) {
                        if max_body < snake_length {
                            movement_position_max_body.insert(movement_position, (snake_id, snake_length));
                        }
                    } else {
                        movement_position_max_body.insert(movement_position, (snake_id, snake_length));
                    }
                }
            }

            (snake_id, snake_length, movement_mask, movement_positions)
        })
        .collect();
    
    // info!("{:?}", movement_position_max_body);
    masks.into_iter()
        .map(|(snake_id, snake_length, mut movement_mask, movement_positions)| {
            for (mask, position) in movement_mask.iter_mut().zip(movement_positions.iter()) {
                if *mask != MovementMask::Safe {
                    continue;
                }
                let &(largest_snake_id, max_length) = movement_position_max_body.get(position).unwrap();
                if snake_id != largest_snake_id && snake_length <= max_length {
                    // info!("{} {:?} {} {:?}", snake_id, position, snake_length, movement_position_max_body.get(position).unwrap());
                    *mask = MovementMask::Risky;
                }
            }
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

pub fn get_best_movement_from_movement_visits(movement_visits: [u32; 4]) -> usize {
    let (movement, _) = movement_visits.iter()
        .enumerate()
        .max_by_key(|(_, &visits)| visits)
        .unwrap_or((0, &0));

    movement
}
