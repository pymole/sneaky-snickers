use crate::{
    game::{
        WIDTH,
        HEIGHT,
        Board,
        MAX_SNAKE_COUNT,
        Point,
        SIZE, CENTER, Snake,
    },
    zobrist::{body_direction, BodyDirections}
};


pub type Rewards = ([f32; MAX_SNAKE_COUNT], bool);

pub fn get_rewards(board: &Board) -> Rewards {
    debug_assert!(board.is_terminal());
    let mut rewards = [0.0; MAX_SNAKE_COUNT];
    let mut draw = true;
    for (i, snake) in board.snakes.iter().enumerate() {
        if snake.is_alive() {
            draw = false;
            rewards[i] = 1.0;
            break;
        }
    }
    (rewards, draw)
}


// NNUE Features

const MAX_BODIES_ON_TAIL: usize = 3;
const ALL_PLAYERS_GRIDS_SIZE: usize = MAX_SNAKE_COUNT * SIZE;
const HEALTH_BARS: usize = 101;

const HEADS_AT: usize = 0;
const TAILS_AT: usize = ALL_PLAYERS_GRIDS_SIZE;
const BODY_PARTS_AT: usize = TAILS_AT + ALL_PLAYERS_GRIDS_SIZE;
const BODIES_ON_TAIL_AT: usize = BODY_PARTS_AT + ALL_PLAYERS_GRIDS_SIZE * 5;
const FOOD_AT: usize = BODIES_ON_TAIL_AT + MAX_BODIES_ON_TAIL * MAX_SNAKE_COUNT;
const HAZARD_AT: usize = FOOD_AT + SIZE;
const HEALTH_AT: usize = HAZARD_AT + SIZE;
pub const TOTAL_FEATURES_SIZE: usize = HEALTH_AT + MAX_SNAKE_COUNT * HEALTH_BARS;


const fn calculate_owner_heads_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_heads_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_heads_at[i] = HEADS_AT + i * SIZE;
        i += 1;
    }
    owner_heads_at
}

const fn calculate_owner_tails_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_tails_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_tails_at[i] = TAILS_AT + i * SIZE;
        i += 1;
    }
    owner_tails_at
}

const fn calculate_owner_body_parts_at() -> [[usize; 5]; MAX_SNAKE_COUNT] {
    let mut owner_body_parts_at = [[0; 5]; MAX_SNAKE_COUNT];
    let mut i = 0;
    let mut d;
    let mut owner_offset;
    while i < MAX_SNAKE_COUNT {
        d = 0;
        owner_offset = BODY_PARTS_AT + i * SIZE * 5;
        while d < 5 {
            owner_body_parts_at[i][d] = owner_offset + d * SIZE;
            d += 1;
        }
        i += 1;
    }
    owner_body_parts_at
}

const fn calculate_owner_bodies_on_tail_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_bodies_on_tail_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_bodies_on_tail_at[i] = BODIES_ON_TAIL_AT + i * MAX_BODIES_ON_TAIL;
        i += 1;
    }
    owner_bodies_on_tail_at
}

const fn calculate_owner_health_at() -> [usize; MAX_SNAKE_COUNT] {
    let mut owner_on_health_at = [0; MAX_SNAKE_COUNT];
    let mut i = 0;
    while i < MAX_SNAKE_COUNT {
        owner_on_health_at[i] = HEALTH_AT + i * HEALTH_BARS;
        i += 1;
    }
    owner_on_health_at
}

const OWNER_HEADS_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_heads_at();
const OWNER_TAILS_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_tails_at();
const OWNER_BODY_PARTS_AT: [[usize; 5]; MAX_SNAKE_COUNT] = calculate_owner_body_parts_at();
const OWNER_BODIES_ON_TAIL_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_bodies_on_tail_at();
const OWNER_HEALTH_AT: [usize; MAX_SNAKE_COUNT] = calculate_owner_health_at();

fn get_head_index(owner: usize, head: Point) -> usize {
    OWNER_HEADS_AT[owner] + get_point_index(head)
}

fn get_tail_index(owner: usize, tail: Point) -> usize {
    OWNER_TAILS_AT[owner] + get_point_index(tail)
}

fn get_body_part_index(owner: usize, body: Point, body_direction: BodyDirections) -> usize {
    OWNER_BODY_PARTS_AT[owner][body_direction as usize] + get_point_index(body)
}

fn get_bodies_on_tail_index(owner: usize, bodies_on_tail: usize) -> usize {
    OWNER_BODIES_ON_TAIL_AT[owner] + bodies_on_tail - 1
}

fn get_food_index(food: Point) -> usize {
    FOOD_AT + get_point_index(food)
}

fn get_hazard_index(hazard: Point) -> usize {
    HAZARD_AT + get_point_index(hazard)
}

fn get_health_index(owner: usize, health: i32) -> usize {
    OWNER_HEALTH_AT[owner] + health as usize
}

#[derive(Clone)]
pub struct IndicesCollector {
    pub indices: Vec<usize>,
}

impl IndicesCollector {
    pub fn new() -> IndicesCollector {
        IndicesCollector { indices: Vec::new() }
    }

    pub fn health(&mut self, owner: usize, health: i32) {
        self.add(get_health_index(owner, health));
    }

    pub fn head(&mut self, owner: usize, pos: Point) {
        self.add(get_head_index(owner, pos));
    }

    pub fn tail(&mut self, owner: usize, pos: Point) {
        self.add(get_tail_index(owner, pos));
    }

    pub fn body_direction(&mut self, owner: usize, body: Point, body_direction: BodyDirections) {
        self.add(get_body_part_index(owner, body, body_direction));
    }

    pub fn bodies_on_tail(&mut self, owner: usize, bodies_on_tail: usize) {
        self.add(get_bodies_on_tail_index(owner, bodies_on_tail));
    }

    pub fn food(&mut self, pos: Point) {
        self.add(get_food_index(pos));
    }

    pub fn hazard(&mut self, pos: Point) {
        self.add(get_hazard_index(pos));
    }

    fn add(&mut self, i: usize) {
        self.indices.push(i);
    }
}


/// Rotates, flips and permutates snakes
/// Index pattern:
/// | rot                       | rot                       |
/// | flip        | flip        | flip        | flip        |
/// | perm | perm | perm | perm | perm | perm | perm | perm |
/// Rotations: 4
/// Flips on rotation: 3
/// Snake positions permutations: A(n=MAX_SNAKES_COUNT, k=alive_snakes_count)

pub type Example = (Vec<usize>, [f32; MAX_SNAKE_COUNT]);

pub fn collect_examples(board: &Board, rewards: [f32; MAX_SNAKE_COUNT]) -> Vec<Example> {
    let alive_snakes: Vec<(usize, &Snake)> = board.snakes.iter().enumerate().filter(|(_, snake)| snake.is_alive()).collect();
    assert!(!alive_snakes.is_empty());
    // TODO: precalulate permutations
    let alive_permutations = get_permutations(alive_snakes.len());
    let perm_count = alive_permutations.len();

    let mut collectors = vec![IndicesCollector::new(); 12 * perm_count];
    let mut rewards_collectors = vec![[0.0; MAX_SNAKE_COUNT]; 12 * perm_count];

    for (alive_i, (snake_i, snake)) in alive_snakes.into_iter().enumerate() {
        // (owner[snakes], health[101])
        let health = snake.health;
        snake_parameter_filler(
            alive_i,
            &mut collectors,
            &alive_permutations,
            |collector: &mut IndicesCollector, owner: usize| {
                collector.health(owner, health);
            },
        );

        // (owner[snakes], head[grid])
        let head = snake.head();
        snake_point_filler(
            head,
            alive_i,
            &mut collectors,
            &alive_permutations,
            |collector, owner, point| {
                collector.head(owner, point);
            },
        );

        // (owner[snakes], tail[grid])
        snake_point_filler(
            snake.tail(),
            alive_i,
            &mut collectors,
            &alive_permutations,
            |collector, owner, point| {
                collector.tail(owner, point);
            },
        );

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;

                let direction = body_direction(back, front);
                snake_body_part_filler(
                    back,
                    direction,
                    alive_i,
                    &mut collectors,
                    &alive_permutations,
                );
                break;
            }
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            snake_body_part_filler(
                front,
                direction,
                alive_i,
                &mut collectors,
                &alive_permutations,
            );
        }
        snake_body_part_filler(
            head,
            BodyDirections::Still,
            alive_i,
            &mut collectors,
            &alive_permutations,
        );

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = snake.body.len() - nearest_to_head_body_on_tail_index;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        
        snake_parameter_filler(
            alive_i,
            &mut collectors,
            &alive_permutations,
            |collector: &mut IndicesCollector, owner: usize| {
                collector.bodies_on_tail(owner, bodies_on_tail);
            },
        );

        rewards_filler(
            rewards[snake_i],
            alive_i,
            &mut rewards_collectors,
            &alive_permutations,
        );
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        grid_point_filler(
            food,
            alive_permutations.len(),
            &mut collectors,
            |collector, point| {
                collector.food(point);
            }
        )
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                grid_point_filler(
                    p,
                    alive_permutations.len(),
                    &mut collectors,
                    |collector, point| {
                        collector.hazard(point);
                    }
                )
            }
        }
    }

    collectors
        .into_iter()
        .map(|collector| collector.indices)
        .zip(rewards_collectors)
        .collect()
}

fn snake_point_filler(
    point: Point,
    alive_index: usize,
    collectors: &mut Vec<IndicesCollector>,
    permutations: &Vec<Vec<usize>>,
    func: fn(&mut IndicesCollector, usize, Point),
) {
    let perm_count = permutations.len();
    for (rot_i, rot) in rotate_point(point).into_iter().enumerate() {
        let flips_at = rot_i * 3 * perm_count;
        for (flip_i, rot_flip) in flip_point(rot).into_iter().enumerate() {
            let perms_at = flips_at + flip_i * perm_count;
            for perm_i in 0..perm_count {
                let perm = &permutations[perm_i];
                let owner = perm[alive_index];
                let collector = &mut collectors[perms_at + perm_i];
                func(collector, owner, rot_flip);
            }
        }
    }
}

fn snake_parameter_filler<F: Fn(&mut IndicesCollector, usize)>(
    alive_index: usize,
    collectors: &mut Vec<IndicesCollector>,
    permutations: &Vec<Vec<usize>>,
    func: F,
) {
    let perm_count = permutations.len();
    for rot_i in 0..4 {
        let flips_at = rot_i * 3 * perm_count;
        for flip_i in 0..3 {
            let perms_at = flips_at + flip_i * perm_count;
            for perm_i in 0..perm_count {
                let perm = &permutations[perm_i];
                let owner = perm[alive_index];
                let collector = &mut collectors[perms_at + perm_i];
                func(collector, owner);
            }
        }
    }
}

fn snake_body_part_filler(
    point: Point,
    direction: BodyDirections,
    alive_index: usize,
    collectors: &mut Vec<IndicesCollector>,
    permutations: &Vec<Vec<usize>>,
) {
    let perm_count = permutations.len();
    for (rot_i, (rot_point, rot_dir)) in rotate_point(point).into_iter().zip(rotate_body_direction(direction)).enumerate() {
        let flips_at = rot_i * 3 * perm_count;
        for (flip_i, (rot_flip_point, rot_flip_dir)) in flip_point(rot_point).into_iter().zip(flip_body_direction(rot_dir)).enumerate() {
            let perms_at = flips_at + flip_i * perm_count;
            for perm_i in 0..perm_count {
                let perm = &permutations[perm_i];
                let owner = perm[alive_index];
                let collector = &mut collectors[perms_at + perm_i];
                collector.body_direction(owner, rot_flip_point, rot_flip_dir);
            }
        }
    }
}


fn grid_point_filler(
    point: Point,
    permutations_count: usize,
    collectors: &mut Vec<IndicesCollector>,
    func: fn(&mut IndicesCollector, Point),
) {
    for (rot_i, rot) in rotate_point(point).into_iter().enumerate() {
        let flips_at = rot_i * 3 * permutations_count;
        for (flip_i, rot_flip) in flip_point(rot).into_iter().enumerate() {
            let perms_at = flips_at + flip_i * permutations_count;
            // Fill all permutations with the same func
            for perm_i in 0..permutations_count {
                let collector = &mut collectors[perms_at + perm_i];
                func(collector, rot_flip);
            }
        }
    }
}

fn rewards_filler(
    reward: f32,
    alive_index: usize,
    rewards: &mut Vec<[f32; MAX_SNAKE_COUNT]>,
    permutations: &Vec<Vec<usize>>,
) {
    
    let perm_count = permutations.len();
    for rot_i in 0..4 {
        let flips_at = rot_i * 3 * perm_count;
        for flip_i in 0..3 {
            let perms_at = flips_at + flip_i * perm_count;
            for perm_i in 0..perm_count {
                let perm = &permutations[perm_i];
                let owner = perm[alive_index];
                rewards[perms_at + perm_i][owner] = reward;
            }
        }
    }
}

fn get_point_index(p: Point) -> usize {
    (p.y * HEIGHT + p.x) as usize
}

// NOTE: Flips for x and y together are created by 180 rotation so
// we don't need to do x + y flip

fn flip_point(p: Point) -> [Point; 3] {
    [
        Point {x: p.x, y: p.y},                             // original
        Point {x: WIDTH - p.x - 1, y: p.y},                 // x-flip
        Point {x: p.x, y: HEIGHT - p.y - 1},                // y-flip
        // Point {x: WIDTH - p.x - 1, y: HEIGHT - p.y - 1},    // x and y flip
    ]
}

fn rotate_point(p: Point) -> [Point; 4] {
    let diff = Point {x: p.x - CENTER.x, y: p.y - CENTER.y};
    [
        Point {x: p.x, y: p.y},
        Point {x: CENTER.x + 0 * diff.x + 1 * diff.y, y: CENTER.y - 1 * diff.x + 0 * diff.y},  // Units x=(0, -1); y=(1, 0)
        Point {x: CENTER.x - 1 * diff.x + 0 * diff.y, y: CENTER.y + 0 * diff.x - 1 * diff.y},  // Units x=(-1, 0); y=(0, -1)
        Point {x: CENTER.x + 0 * diff.x - 1 * diff.y, y: CENTER.y + 1 * diff.x + 0 * diff.y},  // Units x=(0, 1); y=(-1, 0)
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
    assert!(alive_snakes_count <= MAX_SNAKE_COUNT);

    let mut is_selected = [false; MAX_SNAKE_COUNT];
    let mut permutations = Vec::new();

    let mut seq = vec![0; alive_snakes_count];
    let mut i = 0;
    let mut n = 0;

    loop {
        loop {
            while n == MAX_SNAKE_COUNT {
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

pub fn get_features_indices(board: &Board) -> Vec<usize> {
    let mut collector = IndicesCollector::new();

    for (owner, snake) in board.snakes.iter().enumerate() {
        if !snake.is_alive() {
            continue;
        }

        // (owner[snakes], health[101])
        let health = snake.health;
        collector.health(owner, health);

        // (owner[snakes], head[grid])
        let head = snake.head();
        collector.head(owner, head);

        // (owner[snakes], tail[grid])            
        collector.tail(owner, snake.tail());

        // (owner[snakes], body[grid], body_direction[5])
        let mut nearest_to_head_body_on_tail_index = 1;
        for i in (1..snake.body.len()).rev() {
            // TODO: Save back in outside varible
            let back = snake.body[i];
            let front = snake.body[i - 1];
            if back != front {
                nearest_to_head_body_on_tail_index = i;

                let direction = body_direction(back, front);
                collector.body_direction(owner, back, direction);
                break;
            }
        }
        for i in 1..nearest_to_head_body_on_tail_index {
            // TODO: Save back in outside varible
            let back = snake.body[i + 1];
            let front = snake.body[i];
            let direction = body_direction(back, front);
            collector.body_direction(owner, front, direction);
        }
        collector.body_direction(owner, head, BodyDirections::Still);

        // (bodies_on_tail[3 * snakes], )
        let bodies_on_tail = snake.body.len() - nearest_to_head_body_on_tail_index;
        debug_assert!(bodies_on_tail <= MAX_BODIES_ON_TAIL);
        
        collector.bodies_on_tail(owner, bodies_on_tail);
    }

    // (food[grid], )
    for food in board.foods.iter().copied() {
        collector.food(food);
    }
    
    // (hazard[grid], )
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            let p = Point {x, y};
            if board.is_hazard(p) {
                collector.hazard(p);
            }
        }
    }

    collector.indices
}

#[cfg(test)]
mod tests {
    use crate::{
        zobrist::BodyDirections,
        features::{flip_body_direction, rotate_body_direction, rotate_point, flip_point, get_permutations}, game::Point
    };

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