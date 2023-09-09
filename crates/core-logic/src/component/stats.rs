use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

/// The stats of an entity.
#[derive(Component)]
pub struct Stats {
    /// The innate attributes of the entity, like strength.
    pub attributes: Attributes,
    /// The learned skills of the entity, like cooking.
    pub skills: Skills,
}

impl Stats {
    /// Creates a new set of stats with attributes and skills set to the provided default values.
    pub fn new(default_attribute_value: u32, default_skill_value: u32) -> Stats {
        Stats {
            attributes: Attributes::new(default_attribute_value),
            skills: Skills::new(default_skill_value),
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

/// The innate attributes of an entity, like strength.
pub struct Attributes {
    standard: HashMap<Attribute, u32>,
    custom: HashMap<String, u32>,
}

impl Attributes {
    fn new(default_value: u32) -> Attributes {
        let mut standard = HashMap::new();
        for attribute in Attribute::iter() {
            match attribute {
                Attribute::Custom(_) => (),
                a => {
                    standard.insert(a, default_value);
                }
            }
        }

        Attributes {
            standard,
            custom: HashMap::new(),
        }
    }

    /// Gets the value of the provided attribute.
    pub fn get(&self, attribute: Attribute) -> u32 {
        *match attribute {
            Attribute::Custom(s) => self.custom.get(&s),
            a => self.standard.get(&a),
        }
        .unwrap_or(&0)
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

/// The learned skills of an entity, like cooking.
pub struct Skills {
    standard: HashMap<Skill, u32>,
    custom: HashMap<String, u32>,
}

impl Skills {
    fn new(default_value: u32) -> Skills {
        let mut standard = HashMap::new();
        for skill in Skill::iter() {
            match skill {
                Skill::Custom(_) => (),
                s => {
                    standard.insert(s, default_value);
                }
            }
        }

        Skills {
            standard,
            custom: HashMap::new(),
        }
    }

    /// Gets the value of the provided skill.
    pub fn get(&self, skill: Skill) -> u32 {
        *match skill {
            Skill::Custom(s) => self.custom.get(&s),
            a => self.standard.get(&a),
        }
        .unwrap_or(&0)
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
