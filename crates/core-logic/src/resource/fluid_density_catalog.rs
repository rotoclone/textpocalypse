use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::component::{Density, FluidType};

/// Map of fluid types to their densities.
#[derive(Resource)]
pub struct FluidDensityCatalog {
    standard: HashMap<FluidType, Density>,
    custom: HashMap<String, Density>,
}

impl FluidDensityCatalog {
    /// Creates the default catalog of densities.
    pub fn new() -> FluidDensityCatalog {
        FluidDensityCatalog {
            //TODO use a match somewhere
            standard: [
                (FluidType::Water, Density(1.0)),
                (FluidType::DirtyWater, Density(1.1)),
                (FluidType::Alcohol, Density(0.79)),
            ]
            .into(),
            custom: HashMap::new(),
        }
    }

    /// Sets the density of the provided fluid type.
    pub fn set(&mut self, fluid_type: &FluidType, density: Density) {
        match fluid_type {
            FluidType::Custom(id) => self.custom.insert(id.clone(), density),
            _ => self.standard.insert(fluid_type.clone(), density),
        };
    }

    /// Determines the density for the provided fluid type.
    pub fn get(&self, fluid_type: &FluidType) -> Density {
        match fluid_type {
            FluidType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(fluid_type),
        }
        .cloned()
        .unwrap_or(Density(1.0))
    }
}

/// Gets the density of the provided fluid type.
pub fn get_fluid_density(fluid_type: &FluidType, world: &World) -> Density {
    world.resource::<FluidDensityCatalog>().get(fluid_type)
}
