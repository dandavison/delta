use std::ops::{Index, IndexMut};

/// Represent data related to removed/minus and added/plus lines which
/// can be indexed with [`MinusPlusIndex::{Plus`](MinusPlusIndex::Plus)`,`[`Minus}`](MinusPlusIndex::Minus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinusPlus<T> {
    pub minus: T,
    pub plus: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinusPlusIndex {
    Minus,
    Plus,
}

pub use MinusPlusIndex::*;

impl<T> Index<MinusPlusIndex> for MinusPlus<T> {
    type Output = T;
    fn index(&self, side: MinusPlusIndex) -> &Self::Output {
        match side {
            Minus => &self.minus,
            Plus => &self.plus,
        }
    }
}

impl<T> IndexMut<MinusPlusIndex> for MinusPlus<T> {
    fn index_mut(&mut self, side: MinusPlusIndex) -> &mut Self::Output {
        match side {
            Minus => &mut self.minus,
            Plus => &mut self.plus,
        }
    }
}

impl<T> MinusPlus<T> {
    pub fn new(minus: T, plus: T) -> Self {
        MinusPlus { minus, plus }
    }
}

impl<T: Default> Default for MinusPlus<T> {
    fn default() -> Self {
        Self {
            minus: T::default(),
            plus: T::default(),
        }
    }
}
