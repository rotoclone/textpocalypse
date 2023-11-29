use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    component::{Attribute, Skill},
    swap_tuple::swapped,
};

/// Map of skills to their base attributes.
#[derive(Resource)]
pub struct SkillBaseAttributeCatalog {
    default: Attribute,
    standard: HashMap<Skill, Attribute>,
    custom: HashMap<String, Attribute>,
}

impl SkillBaseAttributeCatalog {
    /// Creates the default catalog of base attributes.
    pub fn new() -> SkillBaseAttributeCatalog {
        SkillBaseAttributeCatalog {
            default: Attribute::Strength,
            standard: build_standard_base_attributes(),
            custom: HashMap::new(),
        }
    }

    /// Sets the base attribute of the provided skill.
    pub fn set(&mut self, skill: &Skill, base_attribute: Attribute) {
        match skill {
            Skill::Custom(id) => self.custom.insert(id.clone(), base_attribute),
            _ => self.standard.insert(skill.clone(), base_attribute),
        };
    }

    /// Determines the base attribute for the provided skill.
    pub fn get(&self, skill: &Skill) -> Attribute {
        match skill {
            Skill::Custom(id) => self.custom.get(id),
            _ => self.standard.get(skill),
        }
        .cloned()
        .unwrap_or_else(|| self.default.clone())
    }
}

/// Builds the default base attributes of standard skills.
fn build_standard_base_attributes() -> HashMap<Skill, Attribute> {
    Skill::iter()
        .map(|skill| swapped(get_default_base_attribute(&skill), skill))
        .collect()
}

/// Gets the default base attribute of a skill.
fn get_default_base_attribute(skill: &Skill) -> Attribute {
    match skill {
        Skill::Firearms => Attribute::Perception,
        Skill::Bows => Attribute::Endurance,
        Skill::Blades => Attribute::Endurance,
        Skill::Bludgeons => Attribute::Strength,
        Skill::Fists => Attribute::Strength,
        Skill::Construction => Attribute::Strength,
        Skill::Crafting => Attribute::Intelligence,
        Skill::Scavenging => Attribute::Perception,
        Skill::Stealth => Attribute::Perception,
        Skill::Medicine => Attribute::Intelligence,
        Skill::Cooking => Attribute::Intelligence,
        Skill::Dodging => Attribute::Perception,
        Skill::Custom(_) => Attribute::Strength,
    }
}

/// Gets the base attribute of the provided skill.
pub fn get_base_attribute(skill: &Skill, world: &World) -> Attribute {
    world.resource::<SkillBaseAttributeCatalog>().get(skill)
}
