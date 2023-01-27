use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::component::FluidType;

/// Map of fluid types to their display names.
#[derive(Resource)]
pub struct FluidNameCatalog {
    standard: HashMap<FluidType, String>,
    custom: HashMap<String, String>,
}

impl FluidNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> FluidNameCatalog {
        FluidNameCatalog {
            //TODO use a match somewhere
            standard: [
                (FluidType::Water, "water".to_string()),
                (FluidType::DirtyWater, "dirty water".to_string()),
                (FluidType::Alcohol, "alcohol".to_string()),
            ]
            .into(),
            custom: HashMap::new(),
        }
    }

    /// Sets the name of the provided fluid type.
    pub fn set(&mut self, fluid_type: &FluidType, name: String) {
        match fluid_type {
            FluidType::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(fluid_type.clone(), name),
        };
    }

    /// Determines the name for the provided fluid type.
    pub fn get(&self, fluid_type: &FluidType) -> String {
        match fluid_type {
            FluidType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(fluid_type),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown fluid".to_string())
    }
}

/// Gets the name of the provided fluid type.
pub fn get_fluid_name(fluid_type: &FluidType, world: &World) -> String {
    world.resource::<FluidNameCatalog>().get(fluid_type)
}
