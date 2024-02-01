use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Div, Sub, SubAssign},
};

use bevy_ecs::prelude::*;
use float_cmp::approx_eq;

use crate::{AttributeDescription, Container, Density, FluidContainer, Volume};

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// The weight of an entity, in kilograms.
#[derive(Debug, Clone, Copy, Component, PartialOrd)]
pub struct Weight(pub f32);

impl Add for Weight {
    type Output = Weight;

    fn add(self, rhs: Self) -> Self::Output {
        Weight(self.0 + rhs.0)
    }
}

impl AddAssign for Weight {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Div for Weight {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Sub for Weight {
    type Output = Weight;

    fn sub(self, rhs: Self) -> Self::Output {
        Weight(self.0 - rhs.0)
    }
}

impl SubAssign for Weight {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sum for Weight {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Weight(iter.map(|x| x.0).sum())
    }
}

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(f32, self.0, other.0)
    }
}

impl Eq for Weight {}

impl Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Weight {
    /// Determines the total weight of an entity.
    pub fn get(entity: Entity, world: &World) -> Weight {
        Weight::get_weight_recursive(entity, world, &mut vec![entity])
    }

    fn get_weight_recursive(
        entity: Entity,
        world: &World,
        contained_entities: &mut Vec<Entity>,
    ) -> Weight {
        let mut weight = if let Some(weight) = world.get::<Weight>(entity) {
            *weight
        } else if let Some(density) = world.get::<Density>(entity) {
            if let Some(volume) = world.get::<Volume>(entity) {
                // entity has density and volume, but no weight, so calculate it
                density.weight_of_volume(*volume)
            } else {
                // entity has no weight, and density but no volume
                Weight(0.0)
            }
        } else {
            // entity has no weight, and no density
            Weight(0.0)
        };

        if let Some(container) = world.get::<Container>(entity) {
            let contained_weight = container
                .entities
                .iter()
                .map(|e| {
                    if contained_entities.contains(e) {
                        panic!("{entity:?} contains itself")
                    }
                    contained_entities.push(*e);
                    Weight::get_weight_recursive(*e, world, contained_entities)
                })
                .sum::<Weight>();

            weight += contained_weight;
        }

        if let Some(container) = world.get::<FluidContainer>(entity) {
            let contained_weight = container.contents.get_total_weight(world);

            weight += contained_weight;
        }

        weight
    }
}

/// Describes the weight of an entity.
#[derive(Debug)]
struct WeightAttributeDescriber;

impl AttributeDescriber for WeightAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if detail_level >= AttributeDetailLevel::Advanced {
            let weight = Weight::get(entity, world);

            vec![AttributeDescription::does(format!("weighs {weight:.2} kg"))]
        } else {
            Vec::new()
        }
    }
}

impl DescribeAttributes for Weight {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(WeightAttributeDescriber)
    }
}
