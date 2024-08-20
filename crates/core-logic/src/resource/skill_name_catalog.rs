use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{component::Skill, swap_tuple::swapped};

/// Map of skills to their display names.
#[derive(Resource)]
pub struct SkillNameCatalog {
    standard: HashMap<Skill, String>,
    custom: HashMap<String, String>,
    name_to_skill: HashMap<String, Skill>,
}

impl SkillNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> SkillNameCatalog {
        let standard_names = build_standard_names();
        let name_to_skill = standard_names
            .iter()
            .map(|(skill, name)| (name.to_lowercase(), skill.clone()))
            .collect();
        SkillNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
            name_to_skill,
        }
    }

    /// Gets the name of the provided skill.
    pub fn get_name(skill: &Skill, world: &World) -> String {
        world.resource::<SkillNameCatalog>().get(skill)
    }

    /// Gets the skill with the provided name, ignoring case, if there is one.
    pub fn get_skill(skill_name: &str, world: &World) -> Option<Skill> {
        let catalog = world.resource::<SkillNameCatalog>();
        catalog
            .name_to_skill
            .get(&skill_name.to_lowercase())
            .cloned()
    }

    /// Sets the name of the provided skill.
    pub fn set(&mut self, skill: &Skill, name: String) {
        self.name_to_skill
            .insert(name.to_lowercase(), skill.clone());

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
        .filter_map(|skill| get_default_name(&skill).map(|name| (skill, name)))
        .collect()
}

/// Gets the default display name of a skill.
fn get_default_name(skill: &Skill) -> Option<String> {
    match skill {
        Skill::Firearms => Some("Firearms"),
        Skill::Bows => Some("Bows"),
        Skill::Blades => Some("Blades"),
        Skill::Bludgeons => Some("Bludgeons"),
        Skill::Fists => Some("Fists"),
        Skill::Construction => Some("Construction"),
        Skill::Craft => Some("Craft"),
        Skill::Scavenge => Some("Scavenge"),
        Skill::Stealth => Some("Stealth"),
        Skill::Medicine => Some("Medicine"),
        Skill::Cook => Some("Cook"),
        Skill::Dodge => Some("Dodge"),
        Skill::Climb => Some("Climb"),
        Skill::Lockpick => Some("Lockpick"),
        Skill::Butchery => Some("Butchery"),
        Skill::Custom(_) => None,
    }
    .map(|s| s.to_string())
}

/// Gets the name of the provided skill.
pub fn get_skill_name(skill: &Skill, world: &World) -> String {
    world.resource::<SkillNameCatalog>().get(skill)
}
