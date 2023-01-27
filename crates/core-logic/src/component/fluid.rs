use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::resource::FluidDensityCatalog;

use super::{Volume, Weight};

/// Marks an entity as a fluid.
#[derive(Component)]
pub struct Fluid {
    pub contents: HashMap<FluidType, Volume>,
}

impl Fluid {
    /// Creates a new empty fluid.
    pub fn new() -> Fluid {
        Fluid {
            contents: HashMap::new(),
        }
    }

    /// Determines the total volume of the fluid.
    pub fn get_total_volume(&self) -> Volume {
        self.contents.values().cloned().sum()
    }

    /// Determines the total weight of the fluid
    pub fn get_total_weight(&self, world: &World) -> Weight {
        let density_catalog = world.resource::<FluidDensityCatalog>();
        self.contents
            .iter()
            .map(|(fluid_type, volume)| {
                let density = density_catalog.get(fluid_type);
                density.weight_of_volume(*volume)
            })
            .sum::<Weight>()
    }

    /// Builds a map of contained fluid types to the fraction of fluid they represent.
    pub fn get_fluid_type_fractions(&self) -> HashMap<FluidType, FluidTypeAmount> {
        let total_volume = self.get_total_volume();

        self.contents
            .iter()
            .map(|(fluid_type, volume)| {
                let amount = FluidTypeAmount {
                    volume: *volume,
                    fraction: *volume / total_volume,
                };
                (fluid_type.clone(), amount)
            })
            .collect()
    }

    /// Reduces the fluid by the provided amount. Returns the actual removed volumes, by fluid type.
    pub fn reduce(&mut self, amount: Volume) -> HashMap<FluidType, Volume> {
        let fluid_fractions = self.get_fluid_type_fractions();
        let fluid_amounts_to_remove = fluid_fractions
            .into_iter()
            .map(|(fluid_type, type_amount)| {
                let to_remove = Volume(amount.0 * type_amount.fraction);
                (fluid_type, to_remove)
            })
            .collect::<HashMap<FluidType, Volume>>();

        let mut fluid_amounts_removed = HashMap::new();
        for (fluid_type, to_remove) in fluid_amounts_to_remove {
            if let Some(volume) = self.contents.get(&fluid_type).copied() {
                if to_remove >= volume {
                    self.contents.remove(&fluid_type);
                    fluid_amounts_removed.insert(fluid_type, volume);
                } else {
                    self.contents.insert(fluid_type.clone(), volume - to_remove);
                    fluid_amounts_removed.insert(fluid_type, to_remove);
                }
            }
        }

        fluid_amounts_removed
    }

    /// Adds the provided fluid amounts to this fluid.
    pub fn increase(&mut self, amounts: &HashMap<FluidType, Volume>) {
        for (fluid_type, amount) in amounts {
            let new_amount = if let Some(volume) = self.contents.get(fluid_type).copied() {
                volume + *amount
            } else {
                *amount
            };

            self.contents.insert(fluid_type.clone(), new_amount);
        }
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
