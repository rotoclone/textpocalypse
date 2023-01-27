use std::collections::HashMap;

use bevy_ecs::prelude::*;

/// The stats f an entity.
#[derive(Component)]
pub struct Stats {
    /// The innate attributes of the entity, like strength.
    attributes: Attributes,
    /// The learned skills of the entity, like cooking.
    skills: Skills,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            attributes: Attributes::new(),
            skills: Skills::new(),
        }
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
}

#[derive(Debug, Clone)]
pub enum Attribute {
    Strength,
    Intelligence,
    Perception,
    Endurance,
    Custom(String),
}

#[derive(Debug, Clone)]
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
