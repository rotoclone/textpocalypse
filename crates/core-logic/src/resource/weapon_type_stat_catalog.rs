use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    component::{Attribute, Skill, Stat, WeaponType},
    swap_tuple::swapped,
};

/// Map of weapon types to stats that give them bonuses.
#[derive(Resource)]
pub struct WeaponTypeStatCatalog {
    standard: HashMap<WeaponType, WeaponTypeStats>,
    custom: HashMap<String, WeaponTypeStats>,
}

/// The stats involved in attacks with a type of weapon.
#[derive(Clone)]
pub struct WeaponTypeStats {
    /// The stat used for initial to-hit rolls.
    pub primary: Stat,
    /// The stat that provides a damage bonus, if any.
    pub damage_bonus: Option<Stat>,
    /// The stat that provides a to-hit bonus, if any.
    pub to_hit_bonus: Option<Stat>,
}

impl Default for WeaponTypeStats {
    fn default() -> Self {
        Self {
            primary: Attribute::Strength.into(),
            damage_bonus: None,
            to_hit_bonus: None,
        }
    }
}

impl WeaponTypeStatCatalog {
    /// Gets the stats associated with the provided weapon type.
    pub fn get_stats(weapon_type: &WeaponType, world: &World) -> WeaponTypeStats {
        world.resource::<WeaponTypeStatCatalog>().get(weapon_type)
    }

    /// Creates the default catalog of stats.
    pub fn new() -> WeaponTypeStatCatalog {
        WeaponTypeStatCatalog {
            standard: build_standard_stats(),
            custom: HashMap::new(),
        }
    }

    /// Sets the associated stats for the provided weapon type.
    pub fn set(&mut self, weapon_type: &WeaponType, stats: WeaponTypeStats) {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.insert(id.clone(), stats),
            _ => self.standard.insert(weapon_type.clone(), stats),
        };
    }

    /// Gets the associated stats for the provided weapon type.
    pub fn get(&self, weapon_type: &WeaponType) -> WeaponTypeStats {
        match weapon_type {
            WeaponType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(weapon_type),
        }
        .cloned()
        .unwrap_or_default()
    }
}

/// Builds the default associated stats for standard weapon types.
fn build_standard_stats() -> HashMap<WeaponType, WeaponTypeStats> {
    WeaponType::iter()
        .map(|weapon_type| swapped(get_default_stats(&weapon_type), weapon_type))
        .collect()
}

/// Gets the default associated stats for a weapon type.
fn get_default_stats(weapon_type: &WeaponType) -> WeaponTypeStats {
    match weapon_type {
        WeaponType::Firearm => WeaponTypeStats {
            primary: Skill::Firearms.into(),
            damage_bonus: None,
            to_hit_bonus: Some(Attribute::Perception.into()),
        },
        WeaponType::Bow => WeaponTypeStats {
            primary: Skill::Bows.into(),
            damage_bonus: None,
            to_hit_bonus: Some(Attribute::Perception.into()),
        },
        WeaponType::Blade => WeaponTypeStats {
            primary: Skill::Blades.into(),
            damage_bonus: Some(Attribute::Strength.into()),
            to_hit_bonus: None,
        },
        WeaponType::Bludgeon => WeaponTypeStats {
            primary: Skill::Bludgeons.into(),
            damage_bonus: Some(Attribute::Strength.into()),
            to_hit_bonus: None,
        },
        WeaponType::Fists => WeaponTypeStats {
            primary: Skill::Fists.into(),
            damage_bonus: Some(Attribute::Strength.into()),
            to_hit_bonus: None,
        },
        WeaponType::Custom(_) => WeaponTypeStats::default(),
    }
}
