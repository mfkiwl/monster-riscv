use std::ops::{Add, Sub};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BitVector {
    pub(crate) value: u64,
}

impl BitVector {
    #[allow(dead_code)]
    pub fn new(value: u64) -> Self {
        BitVector { value }
    }
}

impl Add<BitVector> for BitVector {
    type Output = BitVector;

    fn add(self, other: BitVector) -> Self::Output {
        BitVector::new(self.value + other.value)
    }
}

impl Sub<BitVector> for BitVector {
    type Output = BitVector;

    fn sub(self, other: BitVector) -> Self::Output {
        BitVector::new(self.value - other.value)
    }
}
