//! Common math types and functions.

use std::fmt::{self, Debug, Formatter};

/// A rational number represented as a fraction.
#[derive(Copy, Clone)]
pub struct Rational {
    num: i32,
    den: i32,
}

impl Rational {
    /// Create a new rational number.
    #[inline]
    pub const fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    /// Get the numerator.
    #[inline]
    pub const fn num(&self) -> i32 {
        self.num
    }

    /// Get the denominator.
    #[inline]
    pub const fn den(&self) -> i32 {
        self.den
    }
}

impl Debug for Rational {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.num, self.den)
    }
}
