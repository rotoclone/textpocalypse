use bevy_ecs::prelude::*;

use crate::{
    body_part::{BodyPartType, BodyParts},
    component::{Wearable, WornItems},
    BodyPart, Description,
};

/// The description of the items an entity is wearing.
#[derive(Debug, Clone)]
pub struct WornItemsDescription {
    /// The items being worn.
    pub items: Vec<WornItemDescription>,
    /// The names of all the body parts the wearing entity has.
    pub all_body_parts: Vec<BodyPartDescription>,
    /// The maximum total thickness of items allowed on a single body part.
    pub max_thickness: u32,
}

impl WornItemsDescription {
    /// Creates a worn items description for the provided entity.
    /// Returns `None` if the entity has no `WornItems` component.
    pub fn from_entity(entity: Entity, world: &World) -> Option<WornItemsDescription> {
        if let Some(worn_items) = world.get::<WornItems>(entity) {
            let items = worn_items
                .get_all_items()
                .iter()
                .map(|entity| {
                    WornItemDescription::from_entity(*entity, worn_items, world)
                        .unwrap_or_else(|| panic!("entity {entity:?} should be wearable"))
                })
                .collect();

            let all_body_parts = world
                .get::<BodyParts>(entity)
                .map(|body_parts| {
                    body_parts
                        .get_all()
                        .into_iter()
                        .flat_map(|body_part| BodyPartDescription::from_entity(body_part, world))
                        .collect()
                })
                .unwrap_or_default();

            Some(WornItemsDescription {
                items,
                all_body_parts,
                max_thickness: worn_items.max_thickness,
            })
        } else {
            None
        }
    }
}

/// The description of a single body part.
#[derive(Debug, Clone)]
pub struct BodyPartDescription {
    /// A unique ID to differentiate between different body parts with the same name and type.
    /// This ID may differ between runs, so it should only be used to compare body part descriptions returned together at the same time.
    pub id: u64,
    /// The name of the body part.
    pub name: String,
    /// The type of the body part.
    pub body_part_type: BodyPartType,
}

impl BodyPartDescription {
    /// Creates a body part description for the provided body part.
    /// Returns `None` if the entity isn't a body part.
    pub fn from_entity(entity: Entity, world: &World) -> Option<BodyPartDescription> {
        world
            .get::<BodyPart>(entity)
            .map(|body_part| BodyPartDescription {
                id: entity.to_bits(),
                name: Description::get_name(entity, world).unwrap_or("???".to_string()),
                body_part_type: body_part.part_type.clone(),
            })
    }
}

/// The description of an item being worn.
#[derive(Debug, Clone)]
pub struct WornItemDescription {
    /// The name of the item.
    pub name: String,
    /// The thickness of the item.
    pub thickness: u32,
    /// Descriptions of the body parts the item is covering.
    pub body_parts: Vec<BodyPartDescription>,
}

impl WornItemDescription {
    /// Creates a worn item description for the provided item.
    /// Returns `None` if the entity isn't wearable.
    pub fn from_entity(
        entity: Entity,
        worn_items: &WornItems,
        world: &World,
    ) -> Option<WornItemDescription> {
        world
            .get::<Wearable>(entity)
            .map(|wearable| WornItemDescription {
                name: Description::get_name(entity, world).unwrap_or("???".to_string()),
                thickness: wearable.thickness,
                body_parts: worn_items
                    .get_body_parts_item_is_worn_on(entity)
                    .iter()
                    .flat_map(|body_part| BodyPartDescription::from_entity(*body_part, world))
                    .collect(),
            })
    }
}
