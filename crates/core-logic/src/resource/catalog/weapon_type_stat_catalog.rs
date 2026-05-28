use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{
    component::{Attribute, Skill, Stat, WeaponType},
    resource::catalog::Catalog,
};

/// Map of weapon types to stats that give them bonuses.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(WeaponType)]
pub struct WeaponTypeStatCatalog {
    standard: HashMap<WeaponType, WeaponTypeStats>,
    custom: HashMap<String, WeaponTypeStats>,
}

impl Catalog<WeaponType> for WeaponTypeStatCatalog {
    type V = WeaponTypeStats;

    fn get_default_value(thing: &WeaponType) -> Option<Self::V> {
        match thing {
            WeaponType::Firearm => Some(WeaponTypeStats {
                primary: Skill::Firearms.into(),
                damage_bonus: None,
                to_hit_bonus: Some(Attribute::Perception.into()),
            }),
            WeaponType::Bow => Some(WeaponTypeStats {
                primary: Skill::Bows.into(),
                damage_bonus: None,
                to_hit_bonus: Some(Attribute::Perception.into()),
            }),
            WeaponType::Blade => Some(WeaponTypeStats {
                primary: Skill::Blades.into(),
                damage_bonus: Some(Attribute::Strength.into()),
                to_hit_bonus: None,
            }),
            WeaponType::Bludgeon => Some(WeaponTypeStats {
                primary: Skill::Bludgeons.into(),
                damage_bonus: Some(Attribute::Strength.into()),
                to_hit_bonus: None,
            }),
            WeaponType::Fists => Some(WeaponTypeStats {
                primary: Skill::Fists.into(),
                damage_bonus: Some(Attribute::Strength.into()),
                to_hit_bonus: None,
            }),
            WeaponType::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        WeaponTypeStats::default()
    }
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
