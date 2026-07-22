use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{
    component::{Density, FluidType},
    resource::catalog::Catalog,
};

/// Map of fluid types to their densities.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(FluidType)]
pub struct FluidDensityCatalog {
    standard: HashMap<FluidType, Density>,
    custom: HashMap<String, Density>,
}

impl Catalog<FluidType> for FluidDensityCatalog {
    type V = Density;

    fn get_default_value(thing: &FluidType) -> Option<Self::V> {
        match thing {
            FluidType::Water => Some(Density(1.0)),
            FluidType::DirtyWater => Some(Density(1.1)),
            FluidType::Alcohol => Some(Density(0.79)),
            FluidType::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        Density(0.0)
    }
}
