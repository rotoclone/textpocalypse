use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign},
};

use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// The amount of space an entity takes up.
#[derive(Debug, Clone, Component, PartialEq, PartialOrd)]
pub struct Volume(pub f32);

impl Add for Volume {
    type Output = Volume;

    fn add(self, rhs: Self) -> Self::Output {
        Volume(self.0 + rhs.0)
    }
}

impl AddAssign for Volume {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sum for Volume {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Volume(iter.map(|x| x.0).sum())
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Describes the volume of an entity.
#[derive(Debug)]
struct VolumeAttributeDescriber;

impl AttributeDescriber for VolumeAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if detail_level >= AttributeDetailLevel::Advanced {
            let volume = world.get::<Volume>(entity).cloned().unwrap_or(Volume(0.0));

            vec![AttributeDescription::does(format!(
                "takes up {volume} L of space"
            ))]
        } else {
            Vec::new()
        }
    }
}

impl DescribeAttributes for Volume {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(VolumeAttributeDescriber)
    }
}
