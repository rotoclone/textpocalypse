use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    action::PutAction,
    component::Description,
    find_wearing_entity, get_article_reference_name, get_reference_name,
    notification::{Notification, VerifyResult},
    AttributeDescription, BodyPart, GameMessage,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, DescribeAttributes, VerifyActionNotification,
    Wearable,
};

/// The things an entity is wearing.
#[derive(Component)]
pub struct WornItems {
    /// The maximum total thickness of items on a single body part.
    max_thickness: u32,
    /// The items being worn.
    items: HashMap<BodyPart, Vec<Entity>>,
}

/// An error when trying to wear something.
pub enum WearError {
    /// The entity cannot wear things.
    CannotWear,
    /// The item is not wearable.
    NotWearable,
    /// The entity is already wearing the item.
    AlreadyWorn,
    /// The item is too thick for a body part due to another item on it.
    TooThick(BodyPart, Entity),
}

/// An error when trying to take something off.
pub enum RemoveError {
    /// The entity is not wearing the item.
    NotWorn,
}

impl WornItems {
    /// Creates an empty set of worn items.
    pub fn new(max_thickness: u32) -> WornItems {
        WornItems {
            max_thickness,
            items: HashMap::new(),
        }
    }

    /// Determines whether the provided entity is being worn.
    pub fn is_wearing(&self, entity: Entity) -> bool {
        for worn_items in &mut self.items.values() {
            if worn_items.contains(&entity) {
                return true;
            }
        }

        false
    }

    /// Puts on the provided entity, if possible.
    pub fn try_wear(
        wearing_entity: Entity,
        to_wear: Entity,
        world: &mut World,
    ) -> Result<(), WearError> {
        let mut worn_items = match world.entity_mut(wearing_entity).remove::<WornItems>() {
            Some(w) => w,
            None => return Err(WearError::CannotWear),
        };

        let result = worn_items.try_wear_internal(to_wear, world);

        world.entity_mut(wearing_entity).insert(worn_items);

        result
    }

    /// Puts on the provided entity, if possible.
    fn try_wear_internal(&mut self, entity: Entity, world: &World) -> Result<(), WearError> {
        let wearable = match world.get::<Wearable>(entity) {
            Some(w) => w,
            None => return Err(WearError::NotWearable),
        };

        for body_part in &wearable.body_parts {
            if let Some(already_worn) = self.items.get(body_part) {
                if already_worn.contains(&entity) {
                    return Err(WearError::AlreadyWorn);
                }

                // check total thickness, but only if there's already at least one thing on this body part
                // TODO do this in a validate notification handler for the wear action instead?
                if !already_worn.is_empty() {
                    let total_thickness = already_worn
                        .iter()
                        .map(|e| get_thickness(*e, world))
                        .sum::<u32>();
                    if total_thickness + wearable.thickness > self.max_thickness {
                        return Err(WearError::TooThick(
                            *body_part,
                            // unwrap is safe because we've already checked if `already_worn` is empty
                            *already_worn.last().unwrap(),
                        ));
                    }
                }
            }
        }

        for body_part in &wearable.body_parts {
            self.items.entry(*body_part).or_default().push(entity);
        }

        Ok(())
    }

    /// Removes the provided entity, if possible.
    pub fn remove(&mut self, entity: Entity) -> Result<(), RemoveError> {
        let mut removed = false;
        for worn_items in &mut self.items.values_mut() {
            if worn_items.contains(&entity) {
                worn_items.retain(|e| *e != entity);
                removed = true;
            }
        }

        if removed {
            Ok(())
        } else {
            Err(RemoveError::NotWorn)
        }
    }
}

/// Gets the thickness of the provided entity.
fn get_thickness(entity: Entity, world: &World) -> u32 {
    if let Some(wearable) = world.get::<Wearable>(entity) {
        wearable.thickness
    } else {
        0
    }
}

/// Describes the items being worn by an entity.
#[derive(Debug)]
struct WornItemsAttributeDescriber;

impl AttributeDescriber for WornItemsAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let mut descriptions = Vec::new();
        if let Some(worn_items) = world.get::<WornItems>(entity) {
            let worn_entity_names = worn_items
                .items
                .values()
                .flat_map(|items| items.last())
                .unique()
                .map(|e| get_article_reference_name(*e, world))
                .collect::<Vec<String>>();

            for name in worn_entity_names {
                descriptions.push(AttributeDescription::wears(name))
            }
        }

        descriptions
    }
}

impl DescribeAttributes for WornItems {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(WornItemsAttributeDescriber)
    }
}

// Blocks moving items around if they're being worn
pub fn verify_not_wearing_item_to_put(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let source = notification.contents.source;
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;
    if let Some(wearing_entity) = find_wearing_entity(item, world) {
        let item_name = get_reference_name(item, Some(performing_entity), world);
        let wearer_string = if wearing_entity == source {
            "you're".to_string()
        } else {
            let wearer_name = get_reference_name(wearing_entity, Some(performing_entity), world);
            let is_or_are = if let Some(desc) = world.get::<Description>(wearing_entity) {
                if desc.pronouns.plural {
                    "are"
                } else {
                    "is"
                }
            } else {
                "is"
            };
            format!("{wearer_name} {is_or_are}")
        };

        let message =
            format!("You can't put {item_name} there because {wearer_string} wearing it.",);
        return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
    }

    VerifyResult::valid()
}

//TODO queue up a remove action before dropping a worn item or putting it in a container
