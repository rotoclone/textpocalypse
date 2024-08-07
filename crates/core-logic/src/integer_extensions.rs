pub trait IntegerExtensions<M> {
    /// Multiplies this number by `mult`, and rounds the result back into the same type as the original number.
    /// Saturates at the boundaries of the original number type.
    ///
    /// Depending on the types and values involved, the resultant number might end up identical to the original number.
    /// For example, `1.mul_and_round(1.2)` returns `1`.
    fn mul_and_round(&self, mult: M) -> Self;
}

impl IntegerExtensions<f32> for u32 {
    fn mul_and_round(&self, mult: f32) -> Self {
        (*self as f32 * mult)
            .round()
            .clamp(u32::MIN as f32, u32::MAX as f32) as u32
    }
}

impl IntegerExtensions<f32> for u64 {
    fn mul_and_round(&self, mult: f32) -> Self {
        (*self as f64 * mult as f64)
            .round()
            .clamp(u64::MIN as f64, u64::MAX as f64) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change() {
        assert_eq!(1, 1_u32.mul_and_round(1.0));
        assert_eq!(5, 5_u32.mul_and_round(1.0));

        assert_eq!(1, 1_u64.mul_and_round(1.0));
        assert_eq!(5, 5_u64.mul_and_round(1.0));
    }

    #[test]
    fn rounding_causes_no_change() {
        assert_eq!(1, 1_u32.mul_and_round(1.2));
        assert_eq!(1, 1_u32.mul_and_round(0.9));
        assert_eq!(5, 5_u32.mul_and_round(1.02));
        assert_eq!(5, 5_u32.mul_and_round(0.99));

        assert_eq!(1, 1_u64.mul_and_round(1.2));
        assert_eq!(1, 1_u64.mul_and_round(0.9));
        assert_eq!(5, 5_u64.mul_and_round(1.02));
        assert_eq!(5, 5_u64.mul_and_round(0.99));
    }

    #[test]
    fn change_no_rounding() {
        assert_eq!(3, 2_u32.mul_and_round(1.5));
        assert_eq!(3, 2_u64.mul_and_round(1.5));
    }

    #[test]
    fn change_with_rounding() {
        assert_eq!(3, 2_u32.mul_and_round(1.4));
        assert_eq!(3, 2_u32.mul_and_round(1.6));

        assert_eq!(3, 2_u64.mul_and_round(1.4));
        assert_eq!(3, 2_u64.mul_and_round(1.6));
    }

    #[test]
    fn big_numbers() {
        assert_eq!(u32::MAX, 3_000_000_000_u32.mul_and_round(2.0));
        assert_eq!(6_000_000_000, 3_000_000_000_u64.mul_and_round(2.0));
    }
}
