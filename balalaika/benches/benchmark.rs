use balalaika::board_generator::static_board;
use balalaika::engine::EngineSettings;
use balalaika::engine::Movement;
use balalaika::engine::advance_one_step_with_settings;
use balalaika::engine::food_spawner::get_food_spawn_spots;
use balalaika::engine::safe_zone_shrinker::shrink;
use balalaika::game::Board;
use balalaika::game::GridPoint;
use balalaika::game::HEIGHT;
use balalaika::mcts::utils::get_first_able_actions_from_masks;
use balalaika::nnue::predict;
use balalaika::mcts::seq::SequentialMCTS;
use balalaika::mcts::seq::SequentialMCTSConfig;
use balalaika::mcts::search::Search;
use criterion::{black_box, criterion_group, criterion_main, Criterion};


fn predict_benchmark(c: &mut Criterion) {
    let model = tch::CModule::load("../analysis/weights/main.pt").unwrap();
    let board = static_board();

    let mut group = c.benchmark_group("predict");
    group.sample_size(1000);
    group.bench_function("predict", |b| b.iter(|| predict(black_box(&model), black_box(&board))));
    group.finish();
}

fn mcts_benchmark(c: &mut Criterion) {
    let board = static_board();
    let config = SequentialMCTSConfig::from_env();
    
    let mut group = c.benchmark_group("mcts");
    group.sample_size(1000);
    group.bench_function("mcts 1000", |b| b.iter(|| {
        let mut mcts = SequentialMCTS::new(black_box(config));
        mcts.search(black_box(&board), 1000);
    }));
    group.finish();
}

pub fn static_food_spawner(board: &mut Board) {
    // For engine use only! It changes board.objects internal state
    let mut spawn_spots: Vec<_> = get_food_spawn_spots(board).into_iter().collect();
    spawn_spots.sort_by(|p1, p2| (p1.y * HEIGHT as usize + p1.x).cmp(&(p2.y * HEIGHT as usize + p2.x)));

    if !spawn_spots.is_empty() && (board.foods.len() < 1 || board.turn % 5 == 0) {
        board.put_food(GridPoint::from(*spawn_spots[0]));
    }
}

pub fn static_safe_zone_shrinker(board: &mut Board) {
    if board.turn == 0 || board.turn % 20 != 0 || board.safe_zone.empty() {
        return;
    }
    let side: Movement = Movement::from_usize((board.turn % 20 % 4) as usize);
    shrink(board, side);
}

fn engine_benchmark(c: &mut Criterion) {    
    let mut group = c.benchmark_group("engine");
    group.sample_size(1000);
    group.bench_function("static", |b| b.iter(|| {
        let mut board = static_board();
        let mut settings = EngineSettings {
            food_spawner: &mut static_food_spawner,
            safe_zone_shrinker: &mut static_safe_zone_shrinker,
        };
        while !board.is_terminal() {
            let actions = get_first_able_actions_from_masks(&board);
            advance_one_step_with_settings(&mut board, &mut settings, actions);
        }
    }));
    group.finish();
}


criterion_group!(benches, mcts_benchmark, predict_benchmark, engine_benchmark);
criterion_main!(benches);