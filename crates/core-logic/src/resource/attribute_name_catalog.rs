use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{component::Attribute, swap_tuple::swapped};

/// Map of attributes to their display names.
#[derive(Resource)]
pub struct AttributeNameCatalog {
    standard: HashMap<Attribute, AttributeName>,
    custom: HashMap<String, AttributeName>,
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
        AttributeNameCatalog {
            standard: build_standard_names(),
            custom: HashMap::new(),
        }
    }

    /// Gets the name of the provided attribute.
    pub fn get_name(attribute: &Attribute, world: &World) -> AttributeName {
        world.resource::<AttributeNameCatalog>().get(attribute)
    }

    /// Sets the name of the provided attribute.
    pub fn set(&mut self, attribute: &Attribute, name: AttributeName) {
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
        .map(|attribute| swapped(get_default_name(&attribute), attribute))
        .collect()
}

/// Gets the default display name of an attribute.
fn get_default_name(attribute: &Attribute) -> AttributeName {
    match attribute {
        Attribute::Strength => AttributeName::new("Strength", "Str"),
        Attribute::Agility => AttributeName::new("Agility", "Agi"),
        Attribute::Intelligence => AttributeName::new("Intelligence", "Int"),
        Attribute::Perception => AttributeName::new("Perception", "Per"),
        Attribute::Endurance => AttributeName::new("Endurance", "End"),
        Attribute::Custom(_) => AttributeName::new("_CUSTOM_", "_CUSTOM_"),
    }
}

/// Gets the name of the provided attribute.
pub fn get_attribute_name(attribute: &Attribute, world: &World) -> AttributeName {
    world.resource::<AttributeNameCatalog>().get(attribute)
}
