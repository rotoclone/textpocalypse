use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{
    component::{Wearable, WornItems},
    BodyPart, Description,
};

/// The description of the items an entity is wearing.
#[derive(Debug, Clone)]
pub struct WornItemsDescription {
    /// The items being worn.
    pub items: Vec<WornItemDescription>,
    /// The maximum total thickness of items allowed on a single body part.
    pub max_thickness: u32,
}

impl WornItemsDescription {
    /// Creates a worn items description for the provided worn items.
    pub fn from_worn_items(worn_items: &WornItems, world: &World) -> WornItemsDescription {
        let items = worn_items
            .get_all_items()
            .iter()
            .map(|entity| {
                WornItemDescription::from_entity(*entity, world)
                    .unwrap_or_else(|| panic!("entity {entity:?} should be wearable"))
            })
            .collect();
        WornItemsDescription {
            items,
            max_thickness: worn_items.max_thickness,
        }
    }
}

/// The description of an item being worn.
#[derive(Debug, Clone)]
pub struct WornItemDescription {
    /// The name of the item.
    pub name: String,
    /// The thickness of the item.
    pub thickness: u32,
    /// The body parts the item is covering.
    pub body_parts: HashSet<BodyPart>,
}

impl WornItemDescription {
    /// Creates a worn item description for the provided item.
    /// Returns `None` if the entity isn't wearable.
    pub fn from_entity(entity: Entity, world: &World) -> Option<WornItemDescription> {
        world
            .get::<Wearable>(entity)
            .map(|wearable| WornItemDescription {
                name: Description::get_name(entity, world).unwrap_or("???".to_string()),
                thickness: wearable.thickness,
                body_parts: wearable.body_parts.clone(),
            })
    }
}
