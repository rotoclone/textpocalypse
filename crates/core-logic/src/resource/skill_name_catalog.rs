use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{component::Skill, swap_tuple::swapped};

/// Map of skills to their display names.
#[derive(Resource)]
pub struct SkillNameCatalog {
    standard: HashMap<Skill, String>,
    custom: HashMap<String, String>,
}

impl SkillNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> SkillNameCatalog {
        SkillNameCatalog {
            standard: build_standard_names(),
            custom: HashMap::new(),
        }
    }

    /// Sets the name of the provided skill.
    pub fn set(&mut self, skill: &Skill, name: String) {
        match skill {
            Skill::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(skill.clone(), name),
        };
    }

    /// Determines the name for the provided skill.
    pub fn get(&self, skill: &Skill) -> String {
        match skill {
            Skill::Custom(id) => self.custom.get(id),
            _ => self.standard.get(skill),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown skill".to_string())
    }
}

/// Builds the default display names of standard skills.
fn build_standard_names() -> HashMap<Skill, String> {
    Skill::iter()
        .map(|skill| swapped(get_default_name(&skill), skill))
        .collect()
}

/// Gets the default display name of a skill.
fn get_default_name(skill: &Skill) -> String {
    match skill {
        Skill::Firearms => "Firearms",
        Skill::Bows => "Bows",
        Skill::Blades => "Blades",
        Skill::Bludgeons => "Bludgeons",
        Skill::Fists => "Fists",
        Skill::Construction => "Construction",
        Skill::Craft => "Craft",
        Skill::Scavenge => "Scavenge",
        Skill::Stealth => "Stealth",
        Skill::Medicine => "Medicine",
        Skill::Cook => "Cook",
        Skill::Dodge => "Dodge",
        Skill::Climb => "Climb",
        Skill::Lockpick => "Lockpick",
        Skill::Butchery => "Butchery",
        Skill::Custom(_) => "_CUSTOM_",
    }
    .to_string()
}

/// Gets the name of the provided skill.
pub fn get_skill_name(skill: &Skill, world: &World) -> String {
    world.resource::<SkillNameCatalog>().get(skill)
}
