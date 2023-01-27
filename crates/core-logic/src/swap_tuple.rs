pub trait SwapTuple<First, Second>: Sized {
    /// Swaps the elements in the tuple.
    fn swap(self) -> (Second, First) {
        let tuple = self.into_tuple();
        (tuple.1, tuple.0)
    }

    fn into_tuple(self) -> (First, Second);
}

impl<First, Second> SwapTuple<First, Second> for (First, Second) {
    fn into_tuple(self) -> (First, Second) {
        self
    }
}

/// Returns a tuple of the provided arguments but swapped.
pub fn swapped<First, Second>(first: First, second: Second) -> (Second, First) {
    (second, first)
}
