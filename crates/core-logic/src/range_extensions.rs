use std::ops::RangeInclusive;

pub trait RangeExtensions {
    /// Converts a range of `u32` to this type of range.
    fn from_u32_range(range: RangeInclusive<u32>) -> Self;
    /// Adds a number to the start and end of the range.
    fn add(&self, rhs: f32) -> Self;
    /// Subtracts a number from the start and end of the range.
    fn sub(&self, rhs: f32) -> Self;
    /// Multiplies the start and end of the range by a number.
    fn mult(&self, rhs: f32) -> Self;
    /// Converts the range to be over `u32`s, rounding and saturating at the `u32` bounds.
    fn as_u32_saturating(&self) -> RangeInclusive<u32>;
}

impl RangeExtensions for RangeInclusive<f32> {
    fn from_u32_range(range: RangeInclusive<u32>) -> Self {
        let new_start = *range.start() as f32;
        let new_end = *range.end() as f32;
        new_start..=new_end
    }

    fn add(&self, rhs: f32) -> Self {
        let new_start = *self.start() + rhs;
        let new_end = *self.end() + rhs;
        new_start..=new_end
    }

    fn sub(&self, rhs: f32) -> Self {
        self.add(-rhs)
    }

    fn mult(&self, rhs: f32) -> Self {
        let new_start = *self.start() * rhs;
        let new_end = *self.end() * rhs;
        new_start..=new_end
    }

    fn as_u32_saturating(&self) -> RangeInclusive<u32> {
        let new_start = self.start().round().clamp(0.0, u32::MAX as f32) as u32;
        let new_end = self.end().round().clamp(0.0, u32::MAX as f32) as u32;
        new_start..=new_end
    }
}
