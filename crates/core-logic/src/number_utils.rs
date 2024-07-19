use num::Integer;

pub trait MultiplyAndRound<M> {
    /// Multiplies this number by some other number, and rounds the result back into the same type as the original number.
    /// Panics if the original number can't be converted into `f64`. Saturates at the boundaries of the original number type.
    ///
    /// Depending on the types and values involved, the resultant number might end up identical to the original number.
    /// For example, `1.mul_and_round(1.2)` returns `1`.
    fn mul_and_round(&self, multiplier: M) -> Self;
}

impl<T: Integer + TryInto<f64>> MultiplyAndRound<f32> for T {
    fn mul_and_round(&self, multiplier: f32) -> Self {
        let original_float = f64::try_from(*self).expect("original integer should fit in f64");
        let product = original_float * f64::from(multiplier);
        product
            .round()
            .clamp(T::min_value, T::max_value)
            .try_into()
            .expect("rounded product should fit in original integer type")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change() {
        assert_eq!(1, 1.mul_and_round(1.0));
    }

    #[test]
    fn rounding_causes_no_change() {
        assert_eq!(1, 1.mul_and_round(1.2));
        assert_eq!(1, 1.mul_and_round(0.9));
    }
}
