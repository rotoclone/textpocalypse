use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    component::Skill,
    resource::catalog::{Catalog, CatalogBoilerplate},
};

/// Map of skills to their display names.
#[derive(Resource)]
pub struct SkillNameCatalog {
    standard: HashMap<Skill, String>,
    custom: HashMap<String, String>,
    name_to_skill: HashMap<String, Skill>,
}

impl Catalog<Skill> for SkillNameCatalog {
    type V = String;

    fn get_default_value(thing: &Skill) -> Option<Self::V> {
        match thing {
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

    fn get_not_found_value() -> Self::V {
        "an unknown skill".to_string()
    }
}

impl CatalogBoilerplate<Skill> for SkillNameCatalog {
    fn new() -> SkillNameCatalog {
        let standard_names = Skill::iter()
            .filter_map(|skill| Self::get_default_value(&skill).map(|name| (skill, name)))
            .collect::<HashMap<Skill, String>>();

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

    fn get_value(skill: &Skill, world: &World) -> String {
        world.resource::<SkillNameCatalog>().get(skill)
    }

    fn set(&mut self, skill: &Skill, name: String) {
        self.name_to_skill
            .insert(name.to_lowercase(), skill.clone());

        match skill {
            Skill::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(skill.clone(), name),
        };
    }

    fn get(&self, skill: &Skill) -> String {
        match skill {
            Skill::Custom(id) => self.custom.get(id),
            _ => self.standard.get(skill),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown skill".to_string())
    }
}

impl SkillNameCatalog {
    /// Gets the skill with the provided name, ignoring case, if there is one.
    pub fn get_skill(skill_name: &str, world: &World) -> Option<Skill> {
        let catalog = world.resource::<SkillNameCatalog>();
        catalog
            .name_to_skill
            .get(&skill_name.to_lowercase())
            .cloned()
    }
}
