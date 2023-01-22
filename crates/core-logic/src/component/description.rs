use bevy_ecs::prelude::*;
use log::debug;

use crate::GameMessage;

/// The description of an entity.
#[derive(Component, Debug)]
pub struct Description {
    /// The name of the entity.
    pub name: String,
    /// The name to use when referring to the entity as part of a room description.
    pub room_name: String,
    /// The name to use when referring to multiple instances of the entity.
    pub plural_name: String,
    /// The article to use when referring to the entity (usually "a" or "an").
    pub article: Option<String>,
    /// The pronouns to use when referring to the entity.
    pub pronouns: Pronouns,
    /// The alternate names of the entity.
    pub aliases: Vec<String>,
    /// The description of the entity.
    pub description: String,
    /// Describers for dynamic attributes of the entity.
    pub attribute_describers: Vec<Box<dyn AttributeDescriber>>,
}

#[derive(Debug, Clone)]
pub struct Pronouns {
    /// The personal subject form (e.g. he, she, they)
    pub personal_subject: String,
    /// The personal object form (e.g. him, her, them)
    pub personal_object: String,
    /// The possessive form (e.g. his, hers, their).
    pub possessive: String,
    /// Whether the pronouns are considered to be plural or not.
    pub plural: bool,
}

impl Pronouns {
    /// Creates a set of pronouns.
    pub fn new<T: Into<String>>(
        personal_subject: T,
        personal_object: T,
        possessive: T,
        plural: bool,
    ) -> Pronouns {
        Pronouns {
            personal_subject: personal_subject.into(),
            personal_object: personal_object.into(),
            possessive: possessive.into(),
            plural,
        }
    }

    /// Creates a set of pronouns with forms of "he".
    pub fn he() -> Pronouns {
        Pronouns::new("he", "him", "his", false)
    }

    /// Creates a set of pronouns with forms of "she".
    pub fn she() -> Pronouns {
        Pronouns::new("she", "her", "hers", false)
    }

    /// Creates a set of pronouns with forms of "they".
    pub fn they() -> Pronouns {
        Pronouns::new("they", "them", "their", true)
    }

    /// Creates a set of pronouns with forms of "you".
    pub fn you() -> Pronouns {
        Pronouns::new("you", "you", "your", true)
    }

    /// Creates a set of pronouns with forms of "it".
    pub fn it() -> Pronouns {
        Pronouns::new("it", "it", "its", false)
    }
}

impl Description {
    /// Determines whether the provided input refers to the entity with this description.
    pub fn matches(&self, input: &str) -> bool {
        debug!("Checking if {input:?} matches {self:?}");
        self.name.eq_ignore_ascii_case(input)
            || self.room_name.eq_ignore_ascii_case(input)
            || self
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(input))
    }
}

pub trait AttributeDescriber: Send + Sync + std::fmt::Debug {
    /// Generates descriptions of attributes an entity from the perspective of another entity.
    fn describe(
        &self,
        pov_entity: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription>;
}

/// The level of detail to use for attribute descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeDetailLevel {
    /// Basic details, like whether the entity is open.
    Basic = 0,
    /// Advanced details, like how much the entity weighs.
    Advanced,
}

/// A description of a single attribute of an entity.
#[derive(Debug, Clone)]
pub enum AttributeDescription {
    /// A basic attribute, like the fact that an entity is closed.
    Basic(BasicAttributeDescription),
    /// A description in the form of a game message, like the contents of an entity.
    Message(GameMessage),
}

impl AttributeDescription {
    /// Creates a description of something an entity is, like "closed" or "broken".
    pub fn is(description: String) -> AttributeDescription {
        AttributeDescription::Basic(BasicAttributeDescription {
            attribute_type: AttributeType::Is,
            description,
        })
    }

    /// Creates a description of something an entity does, like "glows" or "makes you feel uneasy".
    pub fn does(description: String) -> AttributeDescription {
        AttributeDescription::Basic(BasicAttributeDescription {
            attribute_type: AttributeType::Does,
            description,
        })
    }

    /// Creates a description of something an entity has, like "3 uses left" or "some bites taken out of it".
    pub fn has(description: String) -> AttributeDescription {
        AttributeDescription::Basic(BasicAttributeDescription {
            attribute_type: AttributeType::Has,
            description,
        })
    }
}

/// A basic description of a single attribute of an entity.
#[derive(Debug, Clone)]
pub struct BasicAttributeDescription {
    /// The type of attribute.
    pub attribute_type: AttributeType,
    /// The descrption of the attribute.
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum AttributeType {
    /// Something the entity is, like "closed" or "broken".
    Is,
    /// Something the entity does, like "glows" or "makes you feel uneasy".
    Does,
    /// Something the entity has, like "3 uses left" or "some bites taken out of it".
    Has,
}

/// Trait for components that have describable attributes.
pub trait DescribeAttributes {
    /// Registers the attribute describer for this component on the provided entity.
    fn register_attribute_describer(entity: Entity, world: &mut World) {
        if let Some(mut description) = world.get_mut::<Description>(entity) {
            description
                .attribute_describers
                .push(Self::get_attribute_describer());
        } else {
            world.entity_mut(entity).insert(Description {
                name: "".to_string(),
                room_name: "".to_string(),
                plural_name: "".to_string(),
                article: None,
                pronouns: Pronouns::it(),
                aliases: Vec::new(),
                description: "".to_string(),
                attribute_describers: vec![Self::get_attribute_describer()],
            });
        }
    }

    /// Returns the `AttributeDescriber` for this component.
    fn get_attribute_describer() -> Box<dyn AttributeDescriber>;
}
