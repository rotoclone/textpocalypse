use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{component::FluidType, resource::catalog::Catalog};

/// Map of fluid types to their display names.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(FluidType)]
pub struct FluidNameCatalog {
    standard: HashMap<FluidType, String>,
    custom: HashMap<String, String>,
}

impl Catalog<FluidType> for FluidNameCatalog {
    type V = String;

    fn get_default_value(thing: &FluidType) -> Option<Self::V> {
        match thing {
            FluidType::Water => Some("water"),
            FluidType::DirtyWater => Some("dirty water"),
            FluidType::Alcohol => Some("alcohol"),
            FluidType::Custom(_) => None,
        }
        .map(|s| s.to_string())
    }

    fn get_not_found_value() -> Self::V {
        "an unknown fluid".to_string()
    }
}
