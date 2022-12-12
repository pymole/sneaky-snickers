use std::mem::{MaybeUninit, self};

use tch;

use crate::features::{get_features_indices, TOTAL_FEATURES_SIZE};
use crate::game::{MAX_SNAKE_COUNT, Board};


pub fn predict(model: &tch::CModule, board: &Board) -> tch::Tensor {
    let features = get_features_indices(&board);
    let indices = tch::Tensor::of_slice(features.as_slice());
    let values = tch::Tensor::f_ones(indices.size().as_slice(), (tch::Kind::Float, tch::Device::Cpu)).unwrap();

    let tensor = tch::Tensor::sparse_coo_tensor_indices_size(
        &indices.unsqueeze(0), 
        &values,
        &[TOTAL_FEATURES_SIZE],
        (tch::Kind::Float, tch::Device::Cpu),
    );
    let x = model.forward_ts(&[tensor]).unwrap();
    x
}

pub fn rewards_from_tensor(tensor: tch::Tensor) -> [f32; MAX_SNAKE_COUNT] {
    let mut rewards: [MaybeUninit<f32>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
    for i in 0..MAX_SNAKE_COUNT {
        rewards[i] = MaybeUninit::new(f32::from(tensor.get(i as i64)));
    }
    unsafe { mem::transmute(rewards) }
}

#[cfg(test)]
mod tests {
    use crate::{board_generator::generate_board, nnue::predict};

    #[test]
    fn test_predict() {
        let model = tch::CModule::load("../analysis/weights/main.pt").unwrap();
        let board = generate_board();
        let x = predict(&model, &board);
        println!("{:?}", x);
    }
}