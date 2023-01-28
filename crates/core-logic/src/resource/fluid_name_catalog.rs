use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{component::FluidType, swap_tuple::swapped};

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
            standard: build_standard_names(),
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

/// Builds the default names of standard fluid types.
fn build_standard_names() -> HashMap<FluidType, String> {
    FluidType::iter()
        .map(|fluid_type| swapped(get_default_name(&fluid_type), fluid_type))
        .collect()
}

/// Gets the default name of a fluid type.
fn get_default_name(fluid_type: &FluidType) -> String {
    match fluid_type {
        FluidType::Water => "water",
        FluidType::DirtyWater => "dirty water",
        FluidType::Alcohol => "alcohol",
        FluidType::Custom(_) => "_CUSTOM_",
    }
    .to_string()
}

/// Gets the name of the provided fluid type.
pub fn get_fluid_name(fluid_type: &FluidType, world: &World) -> String {
    world.resource::<FluidNameCatalog>().get(fluid_type)
}
