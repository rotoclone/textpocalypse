use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Div, Sub, SubAssign},
};

use bevy_ecs::prelude::*;
use float_cmp::approx_eq;

use crate::{
    AttributeDescription, AttributeSection, AttributeSectionName, SectionAttributeDescription,
};

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// The amount of space an entity takes up, in liters.
#[derive(Debug, Clone, Copy, Component, PartialOrd)]
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

impl Div for Volume {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Sub for Volume {
    type Output = Volume;

    fn sub(self, rhs: Self) -> Self::Output {
        Volume(self.0 - rhs.0)
    }
}

impl SubAssign for Volume {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sum for Volume {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Volume(iter.map(|x| x.0).sum())
    }
}

impl PartialEq for Volume {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(f32, self.0, other.0)
    }
}

impl Eq for Volume {}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Volume {
    /// Determines the volume of an entity.
    pub fn get(entity: Entity, world: &World) -> Volume {
        world.get::<Volume>(entity).cloned().unwrap_or(Volume(0.0))
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
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let volume = Volume::get(entity, world);

        vec![AttributeDescription::Section(AttributeSection {
            name: AttributeSectionName::Item,
            attributes: vec![SectionAttributeDescription {
                name: "Volume".to_string(),
                description: format!("{volume:.2} L"),
            }],
        })]
    }
}

impl DescribeAttributes for Volume {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(VolumeAttributeDescriber)
    }
}
