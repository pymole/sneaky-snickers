use std::collections::HashMap;
use std::cell::{RefCell, RefMut};
use rand;
use rand::seq::SliceRandom;

use crate::api::objects::Movement;
use crate::engine::{advance_one_step, Action};
use crate::game::{Board, Object, Point};

#[derive(Clone, Default, Debug)]
struct UCB {
    rewards: [f32; 4],
    visits: [u32; 4],
    // With mask we mark available moves
    mask: [bool; 4],
}

impl UCB {
    fn new(mask: [bool; 4]) -> UCB {
        UCB {
            rewards: [0f32; 4],
            visits: [0u32; 4],
            mask: mask,
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    visits: u32,
    ucb_instances: HashMap<usize, UCB>,
}

impl Node {
    fn new(agents: Vec<usize>, masks: Vec<[bool; 4]>) -> Node {
        Node {
            visits: 0,
            ucb_instances: agents
                .into_iter()
                .zip(masks.into_iter().map(|mask| UCB::new(mask)))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct MCTS {
    c: f32,
    nodes: HashMap<Board, RefCell<Node>>,
}

impl MCTS {
    pub fn new(c: f32) -> MCTS {
        MCTS {
            c,
            nodes: HashMap::new(),
        }
    }

    pub fn get_movement(&self, board: &Board, snake: usize) -> Movement {
        let mut node = self.nodes.get(board).unwrap().borrow_mut();
        // node.ucb_instances
        //     .iter()
        //     .for_each(|(ii, i)| i.visits.iter()
        //     .zip(i.rewards)
        //     .enumerate()
        //     .for_each(|(a, (v, r))| {
        //         info!("{} - {}: r: {}; v: {}", ii, a, r, v);
        //     }));

        let ucb_instance = node.ucb_instances.get_mut(&snake).unwrap();

        let (movement, _) = ucb_instance.visits
            .iter()
            .enumerate()
            .filter(|(movement, _)| ucb_instance.mask[*movement])
            .max_by_key(|(_, &visit_count)| visit_count)
            .unwrap_or((0, &0));

        Movement::from_usize(movement)
    }

    pub fn search(&mut self, board: &Board, iterations_count: u32) {
        // TODO: Искать, пока есть время
        for _i in 0..iterations_count {
            // info!("iteration {}", i);
            self.search_iteration(board);
        }
    }

    fn search_iteration(&mut self, board: &Board) {
        let mut board = board.clone();
        let mut path = Vec::new();

        let c = self.c;
        while let Some(node_cell) = self.nodes.get(&board) {
            // info!("Selection");
            let node = node_cell.borrow_mut();
            // info!("{:?}", node.ucb_instances);

            let joint_action = advance_one_step(&mut board, &mut |i, _| {
                let ucb = &node.ucb_instances[&i];
                let available_moves = (0..4)
                    .filter(|&i| ucb.mask[i])
                    .map(|i| (i, ucb.rewards[i], ucb.visits[i]));

                let (best_action, _) = available_moves.fold((0, 0f32), |(best, best_ucb), (action, r, n)| {
                    let ucb;
                    if n > 0 {
                        let exploit = r/(n as f32);
                        let explore = c * ((node.visits as f32).ln() / n as f32).sqrt();

                        ucb = exploit + explore;
                    } else {
                        ucb = f32::INFINITY;
                    }

                    if ucb > best_ucb {
                        (action, ucb)
                    } else {
                        (best, best_ucb)
                    }
                });

                Action::Move(Movement::from_usize(best_action))
            });

            path.push((node, joint_action));
        }

        let expansion_board = board.clone();
        let rewards = rollout(&mut board);
        self.backpropagate(path, rewards);

        if expansion_board.is_terminal() {
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
                let reward = rewards[snake];
                let movement = movement as usize;

                let ucb = node.ucb_instances.get_mut(&snake).unwrap();
                ucb.rewards[movement] += reward;
                ucb.visits[movement] += 1;
            }
        }
    }
}

fn rollout(board: &mut Board) -> Vec<f32> {
    let random = &mut rand::thread_rng();
    while board.is_terminal() {
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
        advance_one_step(board, &mut |snake, _| *actions.get(&snake).unwrap());
    }

    board.snakes
        .iter()
        .map(|snake| snake.is_alive() as u32 as f32)
        .collect()
}


const MOVEMENTS: [Movement; 4] = [
    Movement::Up,
    Movement::Right,
    Movement::Down,
    Movement::Left,
];


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
                            || board.squares[movement_position].object != Object::BodyPart) {
                        movement_mask[movement as usize] = true;
                    }
                });
            (snake_id, movement_mask)
        })
        .collect()
}

fn get_movement_position(position: Point, movement: Movement) -> Point {
    match movement {
        Movement::Right => Point {x: position.x + 1, y: position.y},
        Movement::Left => Point {x: position.x - 1, y: position.y},
        Movement::Up => Point {x: position.x, y: position.y + 1},
        Movement::Down => Point {x: position.x, y: position.y - 1},
    }
}
