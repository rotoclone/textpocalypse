use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::component::WeaponType;

/// Map of weapon types to their display names.
#[derive(Resource)]
pub struct WeaponTypeNameCatalog {
    standard: HashMap<WeaponType, String>,
    custom: HashMap<String, String>,
}

impl WeaponTypeNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> WeaponTypeNameCatalog {
        let standard_names = build_standard_names();
        WeaponTypeNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
        }
    }

    /// Gets the name of the provided weapon type.
    pub fn get_name(weapon_type: &WeaponType, world: &World) -> String {
        world.resource::<WeaponTypeNameCatalog>().get(weapon_type)
    }

    /// Sets the name of the provided weapon type.
    pub fn set(&mut self, weapon_type: &WeaponType, name: String) {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(weapon_type.clone(), name),
        };
    }

    /// Determines the name for the provided weapon type.
    pub fn get(&self, weapon_type: &WeaponType) -> String {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(weapon_type),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown weapon type".to_string())
    }
}

/// Builds the default display names of standard weapon types.
fn build_standard_names() -> HashMap<WeaponType, String> {
    WeaponType::iter()
        .filter_map(|weapon_type| get_default_name(&weapon_type).map(|name| (weapon_type, name)))
        .collect()
}

/// Gets the default display name of a weapon type.
fn get_default_name(weapon_type: &WeaponType) -> Option<String> {
    match weapon_type {
        WeaponType::Firearm => Some("firearm"),
        WeaponType::Bow => Some("bow"),
        WeaponType::Blade => Some("blade"),
        WeaponType::Bludgeon => Some("bludgeon"),
        WeaponType::Fists => Some("fists"),
        WeaponType::Custom(_) => None,
    }
    .map(|s| s.to_string())
}
