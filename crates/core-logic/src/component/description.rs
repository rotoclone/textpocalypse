use bevy_ecs::prelude::*;
use log::debug;

/// The description of an entity.
#[derive(Component, Debug)]
pub struct Description {
    /// The name of the entity.
    pub name: String,
    /// The name to use when referring to the entity as part of a room description.
    pub room_name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The alternate names of the entity.
    pub aliases: Vec<String>,
    /// The description of the entity.
    pub description: String,
    /// Describers for dynamic attributes of the entity.
    pub attribute_describers: Vec<Box<dyn AttributeDescriber>>,
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
    /// Generates descriptions of attributes of the provided entity.
    fn describe(&self, entity: Entity, world: &World) -> Vec<AttributeDescription>;
}

/// A description of a single attribute of an entity.
#[derive(Debug)]
pub struct AttributeDescription {
    /// The type of attribute.
    pub attribute_type: AttributeType,
    /// The descrption of the attribute.
    pub description: String,
}

#[derive(Debug)]
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
                article: None,
                aliases: Vec::new(),
                description: "".to_string(),
                attribute_describers: vec![Self::get_attribute_describer()],
            });
        }
    }

    /// Returns the `AttributeDescriber` for this component.
    fn get_attribute_describer() -> Box<dyn AttributeDescriber>;
}
