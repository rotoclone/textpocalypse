use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use crate::resource::{get_attribute_name, get_base_attribute, get_skill_name};

/* TODO remove
/// Marker trait for types of stats (i.e. attributes and skills)
pub trait Stat: std::fmt::Debug + Send + Sync {
    /// Gets the value of the stat
    fn get_value(&self, stats: &Stats, world: &World) -> f32;
}
*/

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
    pub fn new(default_attribute_value: u16, default_skill_value: u16) -> Stats {
        Stats {
            attributes: Attributes::new(default_attribute_value),
            skills: Skills::new(default_skill_value),
        }
    }

    /// Sets an attribute to a specific value.
    pub fn set_attribute(&mut self, attribute: &Attribute, value: u16) {
        match attribute {
            Attribute::Custom(s) => self.attributes.custom.insert(s.clone(), value),
            a => self.attributes.standard.insert(a.clone(), value),
        };
    }

    /// Sets a skill to a specific value.
    pub fn set_skill(&mut self, skill: &Skill, value: u16) {
        match skill {
            Skill::Custom(s) => self.skills.custom.insert(s.clone(), value),
            a => self.skills.standard.insert(a.clone(), value),
        };
    }

    /// Gets the total value of a skill, taking its base attribute into account.
    pub fn get_skill_total(&self, skill: &Skill, world: &World) -> f32 {
        let base_skill_value = self.skills.get_base(skill);
        let attribute_bonus = self.get_attribute_bonus(skill, world);

        f32::from(base_skill_value) + attribute_bonus
    }

    /// Determines the bonus to apply to the provided skill based on the value of its base attribute.
    pub fn get_attribute_bonus(&self, skill: &Skill, world: &World) -> f32 {
        let attribute = get_base_attribute(skill, world);
        let attribute_value = self.attributes.get(&attribute);

        f32::from(attribute_value) / 2.0
    }
}

/// The innate attributes of an entity, like strength.
pub struct Attributes {
    standard: HashMap<Attribute, u16>,
    custom: HashMap<String, u16>,
}

impl Attributes {
    fn new(default_value: u16) -> Attributes {
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
    pub fn get(&self, attribute: &Attribute) -> u16 {
        *match attribute {
            Attribute::Custom(s) => self.custom.get(s),
            a => self.standard.get(a),
        }
        .unwrap_or(&0)
    }

    /// Gets all the attributes and their values.
    pub fn get_all(&self) -> Vec<(Attribute, u16)> {
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
    standard: HashMap<Skill, u16>,
    custom: HashMap<String, u16>,
}

impl Skills {
    fn new(default_value: u16) -> Skills {
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

    /// Gets the base value of the provided skill.
    fn get_base(&self, skill: &Skill) -> u16 {
        *match skill {
            Skill::Custom(s) => self.custom.get(s),
            a => self.standard.get(a),
        }
        .unwrap_or(&0)
    }

    /// Gets all the skills and their base values.
    pub fn get_all_base(&self) -> Vec<(Skill, u16)> {
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

/// A stat (i.e. either an attribute or a skill)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stat {
    Attribute(Attribute),
    Skill(Skill),
}

impl From<Skill> for Stat {
    fn from(value: Skill) -> Self {
        Stat::Skill(value)
    }
}

impl From<Attribute> for Stat {
    fn from(value: Attribute) -> Self {
        Stat::Attribute(value)
    }
}

impl Stat {
    /// Gets the value of this stat.
    pub fn get_value(&self, stats: &Stats, world: &World) -> f32 {
        match self {
            Stat::Attribute(attribute) => f32::from(stats.attributes.get(attribute)),
            Stat::Skill(skill) => stats.get_skill_total(skill, world),
        }
    }

    /// Gets the provided entity's value for this stat, if they have stats.
    pub fn get_entity_value(&self, entity: Entity, world: &World) -> Option<f32> {
        world
            .get::<Stats>(entity)
            .map(|stats| self.get_value(stats, world))
    }

    /// Gets the display name of this stat.
    pub fn get_name(&self, world: &World) -> String {
        match self {
            Stat::Attribute(attribute) => get_attribute_name(attribute, world).full,
            Stat::Skill(skill) => get_skill_name(skill, world),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Attribute {
    Strength,
    Agility,
    Intelligence,
    Perception,
    Endurance,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Skill {
    Firearms,
    Bows,
    Blades,
    Bludgeons,
    Fists,
    Construction,
    Craft,
    Scavenge,
    Stealth,
    Medicine,
    Cook,
    Dodge,
    Climb,
    Lockpick,
    Butchery,
    Custom(String),
}
