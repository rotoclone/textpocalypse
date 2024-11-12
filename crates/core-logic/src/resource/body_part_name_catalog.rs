use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::component::WeaponType;

/// Map of body parts to their display names.
#[derive(Resource)]
pub struct BodyPartNameCatalog {
    standard: HashMap<BodyPart, String>,
    custom: HashMap<String, String>,
}

impl BodyPartNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> BodyPartNameCatalog {
        let standard_names = build_standard_names();
        BodyPartNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
        }
    }

    /// Gets the name of the provided body part.
    pub fn get_name(body_part: &BodyPart, world: &World) -> String {
        world.resource::<BodyPartNameCatalog>().get(body_part)
    }

    /// Sets the name of the provided body part.
    pub fn set(&mut self, body_part: &BodyPart, name: String) {
        match body_part {
            BodyPart::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(body_part.clone(), name),
        };
    }

    /// Determines the name for the provided body part.
    pub fn get(&self, body_part: &BodyPart) -> String {
        match body_part {
            BodyPart::Custom(id) => self.custom.get(id),
            _ => self.standard.get(body_part),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown body part".to_string())
    }
}

/// Builds the default display names of standard body parts.
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
