pub trait IntegerExtensions {
    /// Multiplies by `mult`, rounding the result and clamping it to valid values.
    fn mul_and_round(&self, mult: f32) -> Self;
}

impl IntegerExtensions for u32 {
    fn mul_and_round(&self, mult: f32) -> Self {
        (*self as f32 * mult)
            .round()
            .clamp(u32::MIN as f32, u32::MAX as f32) as u32
    }
}
