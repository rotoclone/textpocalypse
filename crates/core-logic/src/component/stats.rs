use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::EnumIter;

/// The stats f an entity.
#[derive(Component)]
pub struct Stats {
    /// The innate attributes of the entity, like strength.
    pub attributes: Attributes,
    /// The learned skills of the entity, like cooking.
    pub skills: Skills,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    /// Creates a new empty set of stats.
    pub fn new() -> Stats {
        Stats {
            attributes: Attributes::new(),
            skills: Skills::new(),
        }
    }

    /// Sets an attribute to a specific value.
    pub fn set_attribute(&mut self, attribute: &Attribute, value: u32) {
        match attribute {
            Attribute::Custom(s) => self.attributes.custom.insert(s.clone(), value),
            a => self.attributes.standard.insert(a.clone(), value),
        };
    }

    /// Sets a skill to a specific value.
    pub fn set_skill(&mut self, skill: &Skill, value: u32) {
        match skill {
            Skill::Custom(s) => self.skills.custom.insert(s.clone(), value),
            a => self.skills.standard.insert(a.clone(), value),
        };
    }
}

pub struct Attributes {
    standard: HashMap<Attribute, u32>,
    custom: HashMap<String, u32>,
}

impl Attributes {
    fn new() -> Attributes {
        Attributes {
            standard: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    /// Gets all the attributes and their values.
    pub fn get_all(&self) -> Vec<(Attribute, u32)> {
        let standards = self
            .standard
            .iter()
            .map(|entry| (entry.0.clone(), *entry.1));

        let customs = self
            .custom
            .iter()
            .map(|entry| (Attribute::Custom(entry.0.clone()), *entry.1));

        standards.chain(customs).collect()
    }
}

pub struct Skills {
    standard: HashMap<Skill, u32>,
    custom: HashMap<String, u32>,
}

impl Skills {
    fn new() -> Skills {
        Skills {
            standard: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    /// Gets all the skills and their values.
    pub fn get_all(&self) -> Vec<(Skill, u32)> {
        let standards = self
            .standard
            .iter()
            .map(|entry| (entry.0.clone(), *entry.1));

        let customs = self
            .custom
            .iter()
            .map(|entry| (Skill::Custom(entry.0.clone()), *entry.1));

        standards.chain(customs).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Attribute {
    Strength,
    Intelligence,
    Perception,
    Endurance,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Skill {
    Construction,
    Crafting,
    Scavenging,
    Stealth,
    Firearms,
    Melee,
    Medicine,
    Cooking,
    Dodging,
    Custom(String),
}
