use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    action::{ActionNotificationSender, PutAction, RemoveAction},
    component::Description,
    find_wearing_entity,
    notification::{Notification, VerifyResult},
    AttributeDescription, BodyPart, GameMessage,
};

use super::{
    ActionQueue, AttributeDescriber, AttributeDetailLevel, BeforeActionNotification,
    DescribeAttributes, Location, VerifyActionNotification, Wearable,
};

/// The things an entity is wearing.
#[derive(Component)]
pub struct WornItems {
    /// The maximum total thickness of items allowed on a single body part.
    /// (This is only relevant when trying to wear something on top of something else; a single wearable item can always be worn regardless of its thickness.)
    pub max_thickness: u32,
    /// The items being worn.
    items: HashMap<BodyPart, Vec<Entity>>,
}

/// An error when trying to wear something.
#[derive(Debug)]
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
        let items = BodyPart::iter()
            .map(|body_part| (body_part, Vec::new()))
            .collect();
        WornItems {
            max_thickness,
            items,
        }
    }

    /// Gets all the entities being worn.
    pub fn get_all_items(&self) -> HashSet<Entity> {
        self.items.values().flatten().cloned().collect()
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
    pub fn wear(
        wearing_entity: Entity,
        to_wear: Entity,
        world: &mut World,
    ) -> Result<(), WearError> {
        let mut worn_items = match world.entity_mut(wearing_entity).take::<WornItems>() {
            Some(w) => w,
            None => return Err(WearError::CannotWear),
        };

        let result = worn_items.wear_internal(to_wear, world);

        world.entity_mut(wearing_entity).insert(worn_items);

        result
    }

    /// Puts on the provided entity, if possible.
    fn wear_internal(&mut self, entity: Entity, world: &World) -> Result<(), WearError> {
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
    pub fn remove(
        wearing_entity: Entity,
        to_remove: Entity,
        world: &mut World,
    ) -> Result<(), RemoveError> {
        let mut worn_items;
        match world.get_mut::<WornItems>(wearing_entity) {
            Some(w) => worn_items = w,
            None => return Err(RemoveError::NotWorn),
        };

        let mut removed = false;
        for items in worn_items.items.values_mut() {
            if items.contains(&to_remove) {
                items.retain(|e| *e != to_remove);
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
                .map(|e| Description::get_article_reference_name(*e, world))
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

/// Attempts to remove wearable entities automatically before an attempt is made to move a worn one.
pub fn auto_remove_on_put(
    notification: &Notification<BeforeActionNotification, PutAction>,
    world: &mut World,
) {
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;
    if let Some(wearing_entity) = find_wearing_entity(item, world) {
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(RemoveAction {
                wearing_entity,
                target: item,
                notification_sender: ActionNotificationSender::new(),
            }),
        );
    }
}

// Blocks moving items around if they're being worn
pub fn verify_not_wearing_item_to_put(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let source = notification.contents.source;
    let destination = notification.contents.destination;
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;
    if let Some(wearing_entity) = find_wearing_entity(item, world) {
        let item_name = Description::get_reference_name(item, Some(performing_entity), world);
        let wearer_string = if wearing_entity == performing_entity {
            "you're".to_string()
        } else {
            let wearer_name =
                Description::get_reference_name(wearing_entity, Some(performing_entity), world);
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

        let action_desc = if source == performing_entity {
            let location = world
                .get::<Location>(performing_entity)
                .expect("Performing entity should have a location");
            if destination == location.id {
                format!("drop {item_name}")
            } else {
                format!("put {item_name} there")
            }
        } else {
            format!("get {item_name}")
        };

        let message = format!("You can't {action_desc} because {wearer_string} wearing it.",);
        return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
    }

    VerifyResult::valid()
}
