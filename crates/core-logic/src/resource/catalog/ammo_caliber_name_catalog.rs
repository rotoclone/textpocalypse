use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{component::AmmoCaliber, resource::catalog::Catalog};

/// Map of ammo calibers to their display names.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(AmmoCaliber)]
pub struct AmmoCaliberNameCatalog {
    standard: HashMap<AmmoCaliber, String>,
    custom: HashMap<String, String>,
}

impl Catalog<AmmoCaliber> for AmmoCaliberNameCatalog {
    type V = String;

    fn get_default_value(thing: &AmmoCaliber) -> Option<Self::V> {
        match thing {
            AmmoCaliber::NineMm => Some("9mm".to_string()),
            AmmoCaliber::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        "unknown caliber".to_string()
    }
}
