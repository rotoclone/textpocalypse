use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::component::FluidType;

/// Map of fluid types to their display names.
#[derive(Resource)]
pub struct FluidNames {
    standard: HashMap<FluidType, String>,
    custom: HashMap<String, String>,
}

impl FluidNames {
    /// Creates the default store of names.
    pub fn new() -> FluidNames {
        FluidNames {
            standard: [
                (FluidType::Water, "water".to_string()),
                (FluidType::DirtyWater, "dirty water".to_string()),
                (FluidType::Alcohol, "alcohol".to_string()),
            ]
            .into(),
            custom: HashMap::new(),
        }
    }

    /// Determines the name for the provided fluid type.
    pub fn for_fluid(&self, fluid_type: &FluidType) -> String {
        match fluid_type {
            FluidType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(fluid_type),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown fluid".to_string())
    }
}
