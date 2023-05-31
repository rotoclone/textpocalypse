use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{get_article_reference_name, AttributeDescription, BodyPart};

use super::{
    AttributeDescriber, AttributeDetailLevel, Container, DescribeAttributes, Location, Wearable,
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
#[derive(Debug)]
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
    pub fn wear(
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
        world.entity_mut(to_wear).remove::<Location>();
        world
            .entity_mut(to_wear)
            .insert(Location::Worn(wearing_entity));

        result
    }

    /// Puts on the provided entity, if possible.
    fn try_wear_internal(&mut self, entity: Entity, world: &World) -> Result<(), WearError> {
        let wearable = match world.get::<Wearable>(entity) {
            Some(w) => w,
            None => return Err(WearError::NotWearable),
        };

        if let Some(Location::Worn(_)) = world.get::<Location>(entity) {
            return Err(WearError::AlreadyWorn);
        }

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
            world.entity_mut(to_remove).remove::<Location>();
            if let Some(mut container) = world.get_mut::<Container>(wearing_entity) {
                container.entities.insert(to_remove);
                world
                    .entity_mut(to_remove)
                    .insert(Location::Container(wearing_entity));
            }
            Ok(())
        } else {
            Err(RemoveError::NotWorn)
        }
    }

    /// Removes all the items worn by the provided entity.
    pub fn remove_all(wearing_entity: Entity, world: &mut World) {
        let items_to_remove;
        if let Some(worn_items) = world.get::<WornItems>(wearing_entity) {
            items_to_remove = worn_items.get_all_items();
        } else {
            items_to_remove = vec![];
        };

        for item in items_to_remove {
            // this is inefficient because it looks up `WornItems` for `wearing_entity` a bunch of times
            Self::remove(wearing_entity, item, world)
                .expect("Worn item should be able to be removed");
        }
    }

    /// Gets all the worn items.
    pub fn get_all_items(&self) -> Vec<Entity> {
        self.items.values().flatten().cloned().unique().collect()
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

//TODO queue up a remove action before dropping a worn item or putting it in a container
