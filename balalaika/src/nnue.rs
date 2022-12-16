use std::cell::RefCell;
use std::mem::{MaybeUninit, self};

use tch;

use crate::features::collector::{Rewards, collect_features, FeaturesHandler};
use crate::features::composite::CompositeFeatures;
use crate::game::{MAX_SNAKE_COUNT, Board};


pub struct Model {
    model: tch::CModule,
    composite_features: RefCell<CompositeFeatures>,
}

impl Model {
    pub fn new(model: tch::CModule, composite_features: CompositeFeatures) -> Model {
        Model {
            model,
            composite_features: RefCell::new(composite_features),
        }
    }
    pub fn predict(&self, board: &Board) -> tch::Tensor {
        let composite_features = &mut *self.composite_features.borrow_mut();
        collect_features(board, composite_features);

        let num_features = composite_features.num_features();
        let (indices, values) = composite_features.pop_features();

        let indices = tch::Tensor::of_slice(indices.as_slice());
        let values = tch::Tensor::of_slice(values.as_slice());

        let tensor = tch::Tensor::sparse_coo_tensor_indices_size(
            &indices.unsqueeze(0), 
            &values,
            &[num_features],
            (tch::Kind::Float, tch::Device::Cpu),
        );
        let x = self.model.forward_ts(&[tensor]).unwrap();
        x
    }
}

pub fn rewards_from_tensor(tensor: tch::Tensor) -> Rewards {
    let mut rewards: [MaybeUninit<f32>; MAX_SNAKE_COUNT] = unsafe { MaybeUninit::uninit().assume_init() };
    for i in 0..MAX_SNAKE_COUNT {
        rewards[i] = MaybeUninit::new(f32::from(tensor.get(i as i64)));
    }
    unsafe { mem::transmute(rewards) }
}

#[cfg(test)]
mod tests {
    use crate::{board_generator::generate_board, nnue::Model, features::composite::CompositeFeatures};

    #[test]
    fn test_predict() {
        let model = Model::new(
            tch::CModule::load("../analysis/weights/main.pt").unwrap(),
            CompositeFeatures::new(vec![String::from("base")]),
        );
        let board = generate_board();
        let x = model.predict(&board);
        println!("{:?}", x);
    }
}