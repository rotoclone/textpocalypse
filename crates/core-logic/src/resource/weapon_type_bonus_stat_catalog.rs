use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    component::{Attribute, Stat, WeaponType},
    swap_tuple::swapped,
};

/// Map of weapon types to stats that give them bonuses.
#[derive(Resource)]
pub struct WeaponTypeBonusStatCatalog {
    standard: HashMap<WeaponType, WeaponBonusStats>,
    custom: HashMap<String, WeaponBonusStats>,
}

/// The stats that provide bonuses to attacks with a weapon.
#[derive(Clone)]
pub struct WeaponBonusStats {
    /// The stat that provides a damage bonus, if any.
    pub damage: Option<Stat>,
    /// The stat that provides a to-hit bonus, if any.
    pub to_hit: Option<Stat>,
}

impl WeaponTypeBonusStatCatalog {
    /// Gets the bonus stats for the provided weapon type.
    pub fn get_bonus_stats(weapon_type: &WeaponType, world: &World) -> WeaponBonusStats {
        world
            .resource::<WeaponTypeBonusStatCatalog>()
            .get(weapon_type)
    }

    /// Creates the default catalog of bonus stats.
    pub fn new() -> WeaponTypeBonusStatCatalog {
        WeaponTypeBonusStatCatalog {
            standard: build_standard_bonuses(),
            custom: HashMap::new(),
        }
    }

    /// Sets the bonus stats for the provided weapon type.
    pub fn set(&mut self, weapon_type: &WeaponType, bonuses: WeaponBonusStats) {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.insert(id.clone(), bonuses),
            _ => self.standard.insert(weapon_type.clone(), bonuses),
        };
    }

    /// Determines the bonus stats for the provided weapon type.
    pub fn get(&self, weapon_type: &WeaponType) -> WeaponBonusStats {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(weapon_type),
        }
        .cloned()
        .unwrap_or(WeaponBonusStats {
            damage: None,
            to_hit: None,
        })
    }
}

/// Builds the default bonus stats for standard weapon types.
fn build_standard_bonuses() -> HashMap<WeaponType, WeaponBonusStats> {
    WeaponType::iter()
        .map(|weapon_type| swapped(get_default_bonus_stats(&weapon_type), weapon_type))
        .collect()
}

/// Gets the default bonus stats for a weapon type.
fn get_default_bonus_stats(weapon_type: &WeaponType) -> WeaponBonusStats {
    match weapon_type {
        WeaponType::Firearm => WeaponBonusStats {
            damage: None,
            to_hit: Some(Attribute::Perception.into()),
        },
        WeaponType::Bow => WeaponBonusStats {
            damage: None,
            to_hit: Some(Attribute::Perception.into()),
        },
        WeaponType::Blade => WeaponBonusStats {
            damage: Some(Attribute::Strength.into()),
            to_hit: None,
        },
        WeaponType::Bludgeon => WeaponBonusStats {
            damage: Some(Attribute::Strength.into()),
            to_hit: None,
        },
        WeaponType::Fists => WeaponBonusStats {
            damage: Some(Attribute::Strength.into()),
            to_hit: None,
        },
        WeaponType::Custom(_) => WeaponBonusStats {
            damage: None,
            to_hit: None,
        },
    }
}
