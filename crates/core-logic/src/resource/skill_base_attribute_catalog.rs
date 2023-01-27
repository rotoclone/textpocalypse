use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::component::{Attribute, Skill};

/// Map of skills to their base attributes.
#[derive(Resource)]
pub struct SkillBaseAttributeCatalog {
    default: Attribute,
    custom: HashMap<String, Attribute>,
}

impl SkillBaseAttributeCatalog {
    /// Creates the default catalog of base attributes.
    pub fn new() -> SkillBaseAttributeCatalog {
        SkillBaseAttributeCatalog {
            default: Attribute::Strength,
            custom: HashMap::new(),
        }
    }

    /// Determines the base attribute for the provided skill.
    pub fn for_skill(&self, skill: &Skill) -> Attribute {
        match skill {
            Skill::Construction => Attribute::Strength,
            Skill::Crafting => Attribute::Intelligence,
            Skill::Scavenging => Attribute::Perception,
            Skill::Stealth => Attribute::Perception,
            Skill::Firearms => Attribute::Perception,
            Skill::Melee => Attribute::Strength,
            Skill::Medicine => Attribute::Intelligence,
            Skill::Cooking => Attribute::Intelligence,
            Skill::Dodging => Attribute::Perception,
            Skill::Custom(id) => self.custom.get(id).unwrap_or(&self.default).clone(),
        }
    }
}

/// Gets the base attribute of the provided skill.
pub fn get_base_attribute(skill: &Skill, world: &World) -> Attribute {
    world
        .resource::<SkillBaseAttributeCatalog>()
        .for_skill(skill)
}
