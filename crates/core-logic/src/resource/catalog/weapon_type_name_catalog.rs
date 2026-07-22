use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{component::WeaponType, resource::catalog::Catalog};

/// Map of weapon types to their display names.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(WeaponType)]
pub struct WeaponTypeNameCatalog {
    standard: HashMap<WeaponType, String>,
    custom: HashMap<String, String>,
}

impl Catalog<WeaponType> for WeaponTypeNameCatalog {
    type V = String;

    fn get_default_value(thing: &WeaponType) -> Option<Self::V> {
        match thing {
            WeaponType::Firearm => Some("firearm"),
            WeaponType::Bow => Some("bow"),
            WeaponType::Blade => Some("blade"),
            WeaponType::Bludgeon => Some("bludgeon"),
            WeaponType::Fists => Some("fists"),
            WeaponType::Custom(_) => None,
        }
        .map(|s| s.to_string())
    }

    fn get_not_found_value() -> Self::V {
        "an unknown weapon type".to_string()
    }
}
