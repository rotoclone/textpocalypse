use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::component::Attribute;

/// Map of attributes to their display names.
#[derive(Resource)]
pub struct AttributeNameCatalog {
    standard: HashMap<Attribute, AttributeName>,
    custom: HashMap<String, AttributeName>,
    full_name_to_attribute: HashMap<String, Attribute>,
}

/// The name of an attribute.
#[derive(Debug, Clone)]
pub struct AttributeName {
    /// The full name of the attribute, e.g. "Strength".
    pub full: String,
    /// The short name of the attribute, e.g. "Str".
    pub short: String,
}

impl AttributeName {
    /// Creates an attribute name.
    pub fn new<T: Into<String>>(full: T, short: T) -> AttributeName {
        AttributeName {
            full: full.into(),
            short: short.into(),
        }
    }
}

impl AttributeNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> AttributeNameCatalog {
        let standard_names = build_standard_names();
        let full_name_to_attribute = standard_names
            .iter()
            .map(|(attribute, name)| (name.full.to_lowercase(), attribute.clone()))
            .collect();

        AttributeNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
            full_name_to_attribute,
        }
    }

    /// Gets the name of the provided attribute.
    pub fn get_name(attribute: &Attribute, world: &World) -> AttributeName {
        world.resource::<AttributeNameCatalog>().get(attribute)
    }

    /// Gets the attribute with the provided name, ignoring case, if there is one.
    pub fn get_attribute(attribute_name: &str, world: &World) -> Option<Attribute> {
        let catalog = world.resource::<AttributeNameCatalog>();
        catalog
            .full_name_to_attribute
            .get(&attribute_name.to_lowercase())
            .cloned()
    }

    /// Sets the name of the provided attribute.
    pub fn set(&mut self, attribute: &Attribute, name: AttributeName) {
        self.full_name_to_attribute
            .insert(name.full.to_lowercase(), attribute.clone());

        match attribute {
            Attribute::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(attribute.clone(), name),
        };
    }

    /// Determines the name for the provided attribute.
    pub fn get(&self, attribute: &Attribute) -> AttributeName {
        match attribute {
            Attribute::Custom(id) => self.custom.get(id),
            _ => self.standard.get(attribute),
        }
        .cloned()
        .unwrap_or_else(|| AttributeName::new("an unknown attribute", "UNKNOWN"))
    }
}

/// Builds the default display names of standard attributes.
fn build_standard_names() -> HashMap<Attribute, AttributeName> {
    Attribute::iter()
        .filter_map(|attribute| get_default_name(&attribute).map(|name| (attribute, name)))
        .collect()
}

/// Gets the default display name of an attribute.
fn get_default_name(attribute: &Attribute) -> Option<AttributeName> {
    match attribute {
        Attribute::Strength => Some(AttributeName::new("Strength", "Str")),
        Attribute::Agility => Some(AttributeName::new("Agility", "Agi")),
        Attribute::Intelligence => Some(AttributeName::new("Intelligence", "Int")),
        Attribute::Perception => Some(AttributeName::new("Perception", "Per")),
        Attribute::Endurance => Some(AttributeName::new("Endurance", "End")),
        Attribute::Custom(_) => None,
    }
}

/// Gets the name of the provided attribute.
pub fn get_attribute_name(attribute: &Attribute, world: &World) -> AttributeName {
    world.resource::<AttributeNameCatalog>().get(attribute)
}
