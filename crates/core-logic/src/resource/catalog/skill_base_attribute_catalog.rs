use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{
    component::{Attribute, Skill},
    resource::catalog::Catalog,
};

/// Map of skills to their base attributes.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(Skill)]
pub struct SkillBaseAttributeCatalog {
    standard: HashMap<Skill, Attribute>,
    custom: HashMap<String, Attribute>,
}

impl Catalog<Skill> for SkillBaseAttributeCatalog {
    type V = Attribute;

    fn get_default_value(thing: &Skill) -> Option<Self::V> {
        match thing {
            Skill::Firearms => Some(Attribute::Perception),
            Skill::Bows => Some(Attribute::Agility),
            Skill::Blades => Some(Attribute::Agility),
            Skill::Bludgeons => Some(Attribute::Strength),
            Skill::Fists => Some(Attribute::Strength),
            Skill::Construction => Some(Attribute::Strength),
            Skill::Craft => Some(Attribute::Intelligence),
            Skill::Scavenge => Some(Attribute::Perception),
            Skill::Stealth => Some(Attribute::Perception),
            Skill::Medicine => Some(Attribute::Intelligence),
            Skill::Cook => Some(Attribute::Intelligence),
            Skill::Dodge => Some(Attribute::Agility),
            Skill::Climb => Some(Attribute::Strength),
            Skill::Lockpick => Some(Attribute::Perception),
            Skill::Butchery => Some(Attribute::Perception),
            Skill::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        Attribute::Strength
    }
}
