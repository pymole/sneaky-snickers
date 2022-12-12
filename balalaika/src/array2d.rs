use std::ops::{Index, IndexMut};

use serde::{Serialize, Deserialize};

use crate::game::{WIDTH, HEIGHT};

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct Array2D<T> {
    pub data: [[T; HEIGHT as usize]; WIDTH as usize],
}

impl<T: Copy> Array2D<T> {
    pub fn init_same(value: T) -> Self {
        let data = [[value; HEIGHT as usize]; WIDTH as usize];
        Array2D {
            data
        }
    }
}

impl<T> Index<(usize, usize)> for Array2D<T> {
    type Output = T;

    fn index<'a>(&'a self, (x, y): (usize, usize)) -> &'a T {
        &self.data[x][y]
    }
}

impl<T> IndexMut<(usize, usize)> for Array2D<T> {
    fn index_mut<'a>(&'a mut self, (x, y): (usize, usize)) -> &'a mut T {
        &mut self.data[x][y]
    }
}
