use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Div, Sub, SubAssign},
};

use bevy_ecs::prelude::*;
use float_cmp::approx_eq;

use super::{Volume, Weight};

/// The density of an entity, in kilograms per liter.
#[derive(Debug, Clone, Copy, Component, PartialOrd)]
pub struct Density(pub f32);

impl Add for Density {
    type Output = Density;

    fn add(self, rhs: Self) -> Self::Output {
        Density(self.0 + rhs.0)
    }
}

impl AddAssign for Density {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Div for Density {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Sub for Density {
    type Output = Density;

    fn sub(self, rhs: Self) -> Self::Output {
        Density(self.0 - rhs.0)
    }
}

impl SubAssign for Density {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sum for Density {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Density(iter.map(|x| x.0).sum())
    }
}

impl PartialEq for Density {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(f32, self.0, other.0)
    }
}

impl Eq for Density {}

impl Display for Density {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Density {
    /// Calculates the weight of something of the provided volume with this density.
    pub fn weight_of_volume(&self, volume: Volume) -> Weight {
        Weight(self.0 * volume.0)
    }
}
