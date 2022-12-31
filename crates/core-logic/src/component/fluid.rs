use std::collections::HashMap;

use bevy_ecs::prelude::*;

use super::{Volume, Weight};

/// Marks an entity as a fluid.
#[derive(Component)]
pub struct Fluid {
    pub contents: HashMap<FluidType, Volume>,
}

impl Fluid {
    /// Determines the total volume of the fluid.
    pub fn get_total_volume(&self) -> Volume {
        self.contents.values().cloned().sum()
    }

    /// Determines the total weight of the fluid
    pub fn get_total_weight(&self) -> Weight {
        Weight(1.0) //TODO actually calculate weight
    }

    /// Builds a map of contained fluid types to the fraction of fluid they represent.
    pub fn get_fluid_type_fractions(&self) -> HashMap<FluidType, FluidTypeAmount> {
        let total_volume = self.get_total_volume();

        self.contents
            .iter()
            .map(|(fluid_type, volume)| {
                let amount = FluidTypeAmount {
                    volume: volume.clone(),
                    fraction: volume.clone() / total_volume.clone(),
                };
                (fluid_type.clone(), amount)
            })
            .collect()
    }
}

/// An amount of a single type of fluid.
pub struct FluidTypeAmount {
    /// The volume of the fluid.
    pub volume: Volume,
    /// The fraction of the total fluid in the container this fluid represents.
    pub fraction: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FluidType {
    Water,
    DirtyWater,
    Alcohol,
    Custom(String),
}
