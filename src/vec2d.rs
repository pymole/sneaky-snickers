use std::ops::{Index, IndexMut};

use crate::game::Point;

pub struct Vec2D<T> {
    pub len1: usize,
    pub len2: usize,
    pub data: Vec<T>,
}

impl<T> Vec2D<T> {
    pub fn init_same(len1: usize, len2: usize, value: T) -> Vec2D<T>
    where
        T: Clone
    {
        assert!(len1 > 0);
        assert!(len2 > 0);

        Vec2D::<T> {
            len1: len1,
            len2: len2,
            data: vec![value; len1 * len2],
        }
    }
}

impl<T> Index<(usize, usize)> for Vec2D<T> {
    type Output = T;

    fn index<'a>(&'a self, (i1, i2): (usize, usize)) -> &'a T {
        &self.data[i1 + i2 * self.len1]
    }
}

impl<T> Index<(i32, i32)> for Vec2D<T> {
    type Output = T;

    fn index<'a>(&'a self, (i1, i2): (i32, i32)) -> &'a T {
        &self[(i1 as usize, i2 as usize)]
    }
}

impl<T> Index<Point> for Vec2D<T> {
    type Output = T;

    fn index<'a>(&'a self, p: Point) -> &'a T {
        &self[(p.x, p.y)]
    }
}

impl<T> IndexMut<(usize, usize)> for Vec2D<T> {
    fn index_mut<'a>(&'a mut self, (i1, i2): (usize, usize)) -> &'a mut T {
        &mut self.data[i1 + i2 * self.len1]
    }
}

impl<T> IndexMut<(i32, i32)> for Vec2D<T> {
    fn index_mut<'a>(&'a mut self, (i1, i2): (i32, i32)) -> &'a mut T {
        &mut self[(i1 as usize, i2 as usize)]
    }
}

impl<T> IndexMut<Point> for Vec2D<T> {
    fn index_mut<'a>(&'a mut self, p: Point) -> &'a mut T {
        &mut self[(p.x, p.y)]
    }
}
