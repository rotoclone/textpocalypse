use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    component::Attribute,
    resource::catalog::{Catalog, CatalogBoilerplate},
};

/// Map of attributes to their display names.
#[derive(Resource)]
pub struct AttributeNameCatalog {
    standard: HashMap<Attribute, AttributeName>,
    custom: HashMap<String, AttributeName>,
    full_name_to_attribute: HashMap<String, Attribute>,
}

impl Catalog<Attribute> for AttributeNameCatalog {
    type V = AttributeName;

    fn get_default_value(thing: &Attribute) -> Option<Self::V> {
        match thing {
            Attribute::Strength => Some(AttributeName::new("Strength", "Str")),
            Attribute::Agility => Some(AttributeName::new("Agility", "Agi")),
            Attribute::Intelligence => Some(AttributeName::new("Intelligence", "Int")),
            Attribute::Perception => Some(AttributeName::new("Perception", "Per")),
            Attribute::Endurance => Some(AttributeName::new("Endurance", "End")),
            Attribute::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        AttributeName::new("an unknown attribute", "UNKNOWN")
    }
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

impl CatalogBoilerplate<Attribute> for AttributeNameCatalog {
    fn new() -> AttributeNameCatalog {
        let standard_names = Attribute::iter()
            .filter_map(|attribute| {
                Self::get_default_value(&attribute).map(|name| (attribute, name))
            })
            .collect::<HashMap<Attribute, AttributeName>>();

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

    fn get_value(attribute: &Attribute, world: &World) -> AttributeName {
        world.resource::<AttributeNameCatalog>().get(attribute)
    }

    fn set(&mut self, attribute: &Attribute, name: AttributeName) {
        self.full_name_to_attribute
            .insert(name.full.to_lowercase(), attribute.clone());

        match attribute {
            Attribute::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(attribute.clone(), name),
        };
    }

    fn get(&self, attribute: &Attribute) -> AttributeName {
        match attribute {
            Attribute::Custom(id) => self.custom.get(id),
            _ => self.standard.get(attribute),
        }
        .cloned()
        .unwrap_or_else(|| AttributeName::new("an unknown attribute", "UNKNOWN"))
    }
}

impl AttributeNameCatalog {
    /// Gets the attribute with the provided name, ignoring case, if there is one.
    pub fn get_attribute(attribute_name: &str, world: &World) -> Option<Attribute> {
        let catalog = world.resource::<AttributeNameCatalog>();
        catalog
            .full_name_to_attribute
            .get(&attribute_name.to_lowercase())
            .cloned()
    }
}
