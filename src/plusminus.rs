use std::ops::{Index, IndexMut};

// Struct to represent data related to removed/minus and added/plus lines
// which can be indexed with PlusMinusIndex::{Minus, Plus}.
#[derive(Debug, PartialEq, Eq)]
pub struct PlusMinus<T> {
    pub minus: T,
    pub plus: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlusMinusIndex {
    Minus,
    Plus,
}

pub use PlusMinusIndex::*;

impl<T> Index<PlusMinusIndex> for PlusMinus<T> {
    type Output = T;
    fn index(&self, side: PlusMinusIndex) -> &Self::Output {
        match side {
            Minus => &self.minus,
            Plus => &self.plus,
        }
    }
}

impl<T> IndexMut<PlusMinusIndex> for PlusMinus<T> {
    fn index_mut(&mut self, side: PlusMinusIndex) -> &mut Self::Output {
        match side {
            Minus => &mut self.minus,
            Plus => &mut self.plus,
        }
    }
}

impl<T> PlusMinus<T> {
    pub fn new(minus: T, plus: T) -> Self {
        PlusMinus { minus, plus }
    }
}

impl<T: Default> Default for PlusMinus<T> {
    fn default() -> Self {
        Self {
            minus: T::default(),
            plus: T::default(),
        }
    }
}
