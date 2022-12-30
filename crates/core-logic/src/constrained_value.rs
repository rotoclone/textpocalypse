use std::ops::{Add, Mul, Sub};

/// A value that cannot go over a maximum or under a minimum.
#[derive(Debug, Clone)]
pub struct ConstrainedValue<
    T: PartialOrd<T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy,
> {
    current: T,
    min: T,
    max: T,
}

impl<T: PartialOrd<T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy>
    ConstrainedValue<T>
{
    /// Creates a value with the current value at the minimum.
    pub fn new_min(min: T, max: T) -> ConstrainedValue<T> {
        if min > max {
            panic!("max must be greater than or equal to min")
        }

        ConstrainedValue {
            current: min,
            min,
            max,
        }
    }

    /// Creates a value with the current value at the maximum.
    pub fn new_max(min: T, max: T) -> ConstrainedValue<T> {
        if min > max {
            panic!("max must be greater than or equal to min")
        }

        ConstrainedValue {
            current: max,
            min,
            max,
        }
    }

    /// Creates a value with the current value set to the provided value.
    pub fn new(current: T, min: T, max: T) -> ConstrainedValue<T> {
        let mut value = Self::new_min(min, max);
        value.set(current);

        value
    }

    /// Changes this value by adding the provided value to it.
    pub fn add(&mut self, to_add: T) {
        let new_value = self.current + to_add;
        self.set(new_value);
    }

    /// Changes this value by subtracting the provided value from it.
    pub fn subtract(&mut self, to_subtract: T) {
        let new_value = self.current - to_subtract;
        self.set(new_value);
    }

    /// Changes this value by multiplying it by the provided value.
    pub fn multiply(&mut self, to_multiply: T) {
        let new_value = self.current * to_multiply;
        self.set(new_value);
    }

    /// Sets this value to the provided value.
    pub fn set(&mut self, new_value: T) {
        if new_value > self.max {
            self.current = self.max;
        } else if new_value < self.min {
            self.current = self.min;
        } else {
            self.current = new_value;
        }
    }

    /// Gets the current value.
    pub fn get(&self) -> T {
        self.current
    }

    /// Gets the maximum allowable value.
    pub fn get_min(&self) -> T {
        self.min
    }

    /// Gets the minimum allowable value.
    pub fn get_max(&self) -> T {
        self.max
    }
}
