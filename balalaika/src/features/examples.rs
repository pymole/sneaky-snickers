use std::mem;

use arrayvec::ArrayVec;

use crate::{game::{Board, self, GridPoint, CENTER, HEIGHT, WIDTH, Point, MAX_SNAKE_COUNT}, zobrist::BodyDirections};

use super::{collector::{SparseCollector, get_rewards, ValueType, IndexType, Rewards}, Example};

pub struct ExamplesContext {
    // rewards with the same size as collectors
    pub rewards: Vec<Rewards>,
    pub actual_rewards: Rewards,
    pub alive_permutations: Vec<Vec<usize>>,
    pub alive_snakes: ArrayVec<usize, MAX_SNAKE_COUNT>,
}

impl ExamplesContext {
    pub fn new() -> ExamplesContext {
        // TODO: ExamplesContext should always be initialized
        ExamplesContext {
            rewards: Vec::new(),
            actual_rewards: [0.0; MAX_SNAKE_COUNT],
            alive_permutations: Vec::new(),
            alive_snakes: ArrayVec::new(),
        }
    }

    pub fn set_actual_rewards(&mut self, board: &Board) {
        let actual_rewards = get_rewards(board).0;
        self.actual_rewards = actual_rewards;
    }

    pub fn set_board(&mut self, board: &Board) {
        self.alive_snakes = (0..MAX_SNAKE_COUNT)
            .filter(|i| board.snakes[*i].is_alive())
            .collect();
        self.alive_permutations = get_permutations(self.alive_snakes.len());
        self.refresh_example_rewards();
    }

    fn refresh_example_rewards(&mut self) {
        let alive_snakes = &self.alive_snakes;
        let perms = &self.alive_permutations;
        let actual_rewards = &self.actual_rewards;

        let perm_count = perms.len();
        let mut rewards = vec![[0.0; game::MAX_SNAKE_COUNT]; 12 * perm_count];

        for rot_i in 0..4 {
            let flips_at = rot_i * 3 * perm_count;
            for flip_i in 0..3 {
                let perms_at = flips_at + flip_i * perm_count;
                for perm_i in 0..perm_count {
                    let perm = &perms[perm_i];
                    for (alive_index, snake_index) in alive_snakes.iter().copied().enumerate() {
                        let owner = perm[alive_index];
                        rewards[perms_at + perm_i][owner] = actual_rewards[snake_index];
                    }
                }
            }
        }
        self.rewards = rewards;
    }

    fn pop_rewards(&mut self) -> Vec<Rewards> {
        mem::replace(&mut self.rewards, Vec::new())
    }
}

pub struct ExamplesCollector {
    collectors: Vec<SparseCollector>,
    pub context: ExamplesContext,
}

impl ExamplesCollector {
    pub fn new(context: ExamplesContext) -> ExamplesCollector {
        // TODO: precalulate permutations
        let permutations_count = context.alive_permutations.len();

        // 3 (orginal, flip x, flip y) * 4 (rotates) * snakes permutations
        let collectors = vec![SparseCollector::new(); 12 * permutations_count];
        let examples_collector = ExamplesCollector {
            collectors,
            context,
        };
        examples_collector
    }

    pub fn alive_snake_point_filler<F: Fn(&mut SparseCollector, usize, GridPoint)>(
        &mut self,
        point: GridPoint,
        alive_index: usize,
        func: F,
    ) {
        let perms = &self.context.alive_permutations;
        let perm_count = perms.len();
        for (rot_i, rot) in rotate_point(point).into_iter().enumerate() {
            let flips_at = rot_i * 3 * perm_count;
            for (flip_i, rot_flip) in flip_point(rot).into_iter().enumerate() {
                let perms_at = flips_at + flip_i * perm_count;
                for perm_i in 0..perm_count {
                    let perm = &perms[perm_i];
                    let owner = perm[alive_index];
                    let collector = &mut self.collectors[perms_at + perm_i];
                    func(collector, owner, rot_flip);
                }
            }
        }
    }

    pub fn alive_snake_parameter_filler<F: Fn(&mut SparseCollector, usize)>(
        &mut self,
        alive_index: usize,
        func: F,
    ) {
        let perms = &self.context.alive_permutations;
        let perm_count = perms.len();
        for rot_i in 0..4 {
            let flips_at = rot_i * 3 * perm_count;
            for flip_i in 0..3 {
                let perms_at = flips_at + flip_i * perm_count;
                for perm_i in 0..perm_count {
                    let perm = &perms[perm_i];
                    let owner = perm[alive_index];
                    let collector = &mut self.collectors[perms_at + perm_i];
                    func(collector, owner);
                }
            }
        }
    }

    pub fn alive_snake_body_part_filler<F: Fn(&mut SparseCollector, usize, GridPoint, BodyDirections)>(
        &mut self,
        point: GridPoint,
        direction: BodyDirections,
        alive_index: usize,
        func: F,
    ) {
        let perms = &self.context.alive_permutations;
        let perm_count = perms.len();
        for (rot_i, (rot_point, rot_dir)) in rotate_point(point).into_iter().zip(rotate_body_direction(direction)).enumerate() {
            let flips_at = rot_i * 3 * perm_count;
            for (flip_i, (rot_flip_point, rot_flip_dir)) in flip_point(rot_point).into_iter().zip(flip_body_direction(rot_dir)).enumerate() {
                let perms_at = flips_at + flip_i * perm_count;
                for perm_i in 0..perm_count {
                    let perm = &perms[perm_i];
                    let owner = perm[alive_index];
                    let collector = &mut self.collectors[perms_at + perm_i];
                    func(collector, owner, rot_flip_point, rot_flip_dir);
                }
            }
        }
    }


    pub fn grid_point_filler<F: Fn(&mut SparseCollector, GridPoint)>(
        &mut self,
        point: GridPoint,
        func: F,
    ) {
        let permutations_count = self.context.alive_permutations.len();
        for (rot_i, rot) in rotate_point(point).into_iter().enumerate() {
            let flips_at = rot_i * 3 * permutations_count;
            for (flip_i, rot_flip) in flip_point(rot).into_iter().enumerate() {
                let perms_at = flips_at + flip_i * permutations_count;
                // Fill all permutations with the same func
                for perm_i in 0..permutations_count {
                    let collector = &mut self.collectors[perms_at + perm_i];
                    func(collector, rot_flip);
                }
            }
        }
    }

    pub fn alive_snakes_permutations_filler<F: Fn(&mut SparseCollector, &Vec<usize>)>(
        &mut self,
        func: F,
    ) {
        // Use in case you want to permutate array of all snakes with their relations to others snakes.
        // 4 snakes, each has 3 relations.
        let perms = &self.context.alive_permutations;
        let perm_count = perms.len();
        for rot_i in 0..4 {
            let flips_at = rot_i * 3 * perm_count;
            for flip_i in 0..3 {
                let perms_at = flips_at + flip_i * perm_count;
                for perm_i in 0..perm_count {
                    let perm = &perms[perm_i];
                    let collector = &mut self.collectors[perms_at + perm_i];
                    func(collector, perm);
                }
            }
        }
    }

    // TODO: rename to just parameter
    pub fn parameter(
        &mut self,
        index: IndexType,
        value: ValueType,
    ) {
        for collector in &mut self.collectors {
            collector.add(index, value);
        }
    }

    pub fn len(&self) -> usize {
        self.collectors.len()
    }

    pub fn pop_examples(&mut self) -> Vec<Example> {
        let mut examples = Vec::new();
        for (collector, rewards) in self.collectors.iter_mut().zip(self.context.pop_rewards()) {
            let indices = mem::replace(&mut collector.indices, Vec::new());
            let values = mem::replace(&mut collector.values, Vec::new());
            examples.push(((indices, values), rewards));
        }
        examples
    }

    pub fn refresh_collectors(&mut self) {
        let permutations_count = self.context.alive_permutations.len();

        // 3 (orginal, flip x, flip y) * 4 (rotates) * snakes permutations
        self.collectors = vec![SparseCollector::new(); 12 * permutations_count];
    }
}


// NOTE: Flips for x and y together are created by 180 rotation so
// we don't need to do x + y flip

fn flip_point(p: GridPoint) -> [GridPoint; 3] {
    [
        Point {x: p.x, y: p.y},                             // original
        Point {x: WIDTH - p.x - 1, y: p.y},                 // x-flip
        Point {x: p.x, y: HEIGHT - p.y - 1},                // y-flip
        // Point {x: WIDTH - p.x - 1, y: HEIGHT - p.y - 1},    // x and y flip
    ]
}

fn rotate_point(p: GridPoint) -> [GridPoint; 4] {
    let diff = Point {x: p.x - CENTER.x, y: p.y - CENTER.y};
    [
        GridPoint {x: p.x, y: p.y},
        GridPoint {x: CENTER.x + 0 * diff.x + 1 * diff.y, y: CENTER.y - 1 * diff.x + 0 * diff.y},  // Units x=(0, -1); y=(1, 0)
        GridPoint {x: CENTER.x - 1 * diff.x + 0 * diff.y, y: CENTER.y + 0 * diff.x - 1 * diff.y},  // Units x=(-1, 0); y=(0, -1)
        GridPoint {x: CENTER.x + 0 * diff.x - 1 * diff.y, y: CENTER.y + 1 * diff.x + 0 * diff.y},  // Units x=(0, 1); y=(-1, 0)
    ]
}

fn flip_body_direction(body_direction: BodyDirections) -> [BodyDirections; 3] {
    if body_direction == BodyDirections::Still {
        return [BodyDirections::Still; 3];
    }
    let b = body_direction as usize;
    [
        BodyDirections::from(b),                                    // original
        BodyDirections::from((b + 2 * (1 - (b + 1) % 2)) % 4),      // x-flip
        BodyDirections::from((b + 2 * ((b + 1) % 2)) % 4),          // y-flip
        // BodyDirections::from((b + 2) % 4),                          // x and y flip
    ]
}

fn rotate_body_direction(body_direction: BodyDirections) -> [BodyDirections; 4] {
    if body_direction == BodyDirections::Still {
        return [BodyDirections::Still; 4];
    }
    let b = body_direction as usize;
    [
        BodyDirections::from(b),
        BodyDirections::from((b + 1) % 4),
        BodyDirections::from((b + 2) % 4),
        BodyDirections::from((b + 3) % 4),
    ]
}

fn get_permutations(alive_snakes_count: usize) -> Vec<Vec<usize>> {
    assert!(alive_snakes_count <= game::MAX_SNAKE_COUNT);

    let mut is_selected = [false; game::MAX_SNAKE_COUNT];
    let mut permutations = Vec::new();

    let mut seq = vec![0; alive_snakes_count];
    let mut i = 0;
    let mut n = 0;

    loop {
        loop {
            while n == game::MAX_SNAKE_COUNT {
                if i == 0 {
                    return permutations;
                }
                is_selected[seq[i]] = false;
                i -= 1;
                n = seq[i];
                is_selected[n] = false;
                n += 1;
            }

            if !is_selected[n] {
                break;
            }
            
            n += 1;
        }

        seq[i] = n;

        if i == alive_snakes_count - 1 {
            permutations.push(seq.clone());
            n += 1;
        } else {
            is_selected[n] = true;
            i += 1;
            n = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{flip_body_direction, rotate_body_direction, rotate_point, flip_point, get_permutations};
    use crate::game::Point;
    use crate::zobrist::BodyDirections;

    #[test]
    fn test_rotate_point() {
        assert_eq!(rotate_point(Point {x: 10, y: 10}), [Point {x: 10, y: 10}, Point {x: 10, y: 0}, Point {x: 0, y: 0}, Point {x: 0, y: 10}]);
        assert_eq!(rotate_point(Point {x: 9, y: 2}), [Point {x: 9, y: 2}, Point {x: 2, y: 1}, Point {x: 1, y: 8}, Point {x: 8, y: 9}]);
        assert_eq!(rotate_point(Point {x: 5, y: 5}), [Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}]);
    }

    #[test]
    fn test_flip_point() {
        assert_eq!(flip_point(Point {x: 10, y: 10}), [Point {x: 10, y: 10}, Point {x: 0, y: 10}, Point {x: 10, y: 0}]);
        assert_eq!(flip_point(Point {x: 9, y: 2}), [Point {x: 9, y: 2}, Point {x: 1, y: 2}, Point {x: 9, y: 8}]);
        assert_eq!(flip_point(Point {x: 5, y: 5}), [Point {x: 5, y: 5}, Point {x: 5, y: 5}, Point {x: 5, y: 5}]);
    }

    #[test]
    fn test_flip_body_direction() {
        assert_eq!(flip_body_direction(BodyDirections::Up), [BodyDirections::Up, BodyDirections::Up, BodyDirections::Down]);
        assert_eq!(flip_body_direction(BodyDirections::Right), [BodyDirections::Right, BodyDirections::Left, BodyDirections::Right]);
        assert_eq!(flip_body_direction(BodyDirections::Down), [BodyDirections::Down, BodyDirections::Down, BodyDirections::Up]);
        assert_eq!(flip_body_direction(BodyDirections::Left), [BodyDirections::Left, BodyDirections::Right, BodyDirections::Left]);
        assert_eq!(flip_body_direction(BodyDirections::Still), [BodyDirections::Still, BodyDirections::Still, BodyDirections::Still]);
    }

    #[test]
    fn test_rotate_body_direction() {
        assert_eq!(rotate_body_direction(BodyDirections::Up), [BodyDirections::Up, BodyDirections::Right, BodyDirections::Down, BodyDirections::Left]);
        assert_eq!(rotate_body_direction(BodyDirections::Right), [BodyDirections::Right, BodyDirections::Down, BodyDirections::Left, BodyDirections::Up]);
        assert_eq!(rotate_body_direction(BodyDirections::Down), [BodyDirections::Down, BodyDirections::Left, BodyDirections::Up, BodyDirections::Right]);
        assert_eq!(rotate_body_direction(BodyDirections::Left), [BodyDirections::Left, BodyDirections::Up, BodyDirections::Right, BodyDirections::Down]);
        assert_eq!(rotate_body_direction(BodyDirections::Still), [BodyDirections::Still, BodyDirections::Still, BodyDirections::Still, BodyDirections::Still]);    
    }

    #[test]
    fn test_permutations() {
        assert_eq!(get_permutations(1), vec![
            vec![0],
            vec![1],
            vec![2],
            vec![3],
        ]);
        assert_eq!(get_permutations(2), vec![
            vec![0, 1],
            vec![0, 2],
            vec![0, 3],
            vec![1, 0],
            vec![1, 2],
            vec![1, 3],
            vec![2, 0],
            vec![2, 1],
            vec![2, 3],
            vec![3, 0],
            vec![3, 1],
            vec![3, 2],
        ]);
        assert_eq!(get_permutations(4), vec![
            vec![0, 1, 2, 3], vec![0, 1, 3, 2], vec![0, 2, 1, 3], vec![0, 2, 3, 1],
            vec![0, 3, 1, 2], vec![0, 3, 2, 1], vec![1, 0, 2, 3], vec![1, 0, 3, 2],
            vec![1, 2, 0, 3], vec![1, 2, 3, 0], vec![1, 3, 0, 2], vec![1, 3, 2, 0],
            vec![2, 0, 1, 3], vec![2, 0, 3, 1], vec![2, 1, 0, 3], vec![2, 1, 3, 0],
            vec![2, 3, 0, 1], vec![2, 3, 1, 0], vec![3, 0, 1, 2], vec![3, 0, 2, 1],
            vec![3, 1, 0, 2], vec![3, 1, 2, 0], vec![3, 2, 0, 1], vec![3, 2, 1, 0],
        ]);
    }
}