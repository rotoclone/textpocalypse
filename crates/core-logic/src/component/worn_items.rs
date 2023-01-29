use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::BodyPart;

use super::Wearable;

/// The things an entity is wearing.
#[derive(Component)]
pub struct WornItems {
    /// The maximum total thickness of items on a single body part.
    max_thickness: u32,
    /// The items being worn.
    items: HashMap<BodyPart, Vec<Entity>>,
}

pub enum WearError {
    NotWearable,
    AlreadyWorn,
    TooThick(BodyPart),
}

pub enum RemoveError {
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
    pub fn try_wear(&mut self, entity: Entity, world: &World) -> Result<(), WearError> {
        let wearable = match world.get::<Wearable>(entity) {
            Some(w) => w,
            None => return Err(WearError::NotWearable),
        };

        for body_part in &wearable.body_parts {
            if let Some(already_worn) = self.items.get(body_part) {
                // check total thickness, but only if there's already at least one thing on this body part
                if !already_worn.is_empty() {
                    let total_thickness = already_worn
                        .iter()
                        .map(|e| get_thickness(*e, world))
                        .sum::<u32>();
                    if total_thickness + wearable.thickness > self.max_thickness {
                        return Err(WearError::TooThick(*body_part));
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
