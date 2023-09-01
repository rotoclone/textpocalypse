use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::{
    action::{PutAction, WearAction},
    find_holding_entity, get_article_reference_name,
    notification::Notification,
    AttributeDescription,
};

use super::{
    get_hands_to_hold, AttributeDescriber, AttributeDetailLevel, BeforeActionNotification,
    DescribeAttributes,
};

/// The things an entity is holding.
#[derive(Component)]
pub struct HeldItems {
    /// The number of hands the entity can hold things in.
    pub hands: u8,
    /// The items being held.
    items: Vec<Entity>,
}

/// An error when trying to hold something.
#[derive(Debug)]
pub enum HoldError {
    /// The entity cannot hold things.
    CannotHold,
    /// The item in question cannot be held.
    CannotBeHeld,
    /// The entity is already holding the item.
    AlreadyHeld,
    /// The entity doesn't have enough free hands to hold the item.
    NotEnoughHands,
}

/// An error when trying to stop holding something.
#[derive(Debug)]
pub enum UnholdError {
    /// The entity is not holding the item.
    NotHolding,
}

impl HeldItems {
    /// Creates a `HeldItems` with the provided number of hands.
    pub fn new(hands: u8) -> HeldItems {
        HeldItems {
            hands,
            items: Vec::new(),
        }
    }

    /// Determines whether the provided entity is being held.
    pub fn is_holding(&self, entity: Entity) -> bool {
        self.items.contains(&entity)
    }

    /// Determines how many hands are currently holding items.
    pub fn get_num_hands_used(&self, world: &World) -> u8 {
        self.items
            .iter()
            .flat_map(|item| get_hands_to_hold(*item, world).map(|h: NonZeroU8| h.get()))
            .sum()
    }

    /// Returns the item that has been held the longest, skipping the provided number of items, if there is one.
    pub fn get_oldest_item(&self, to_skip: usize) -> Option<Entity> {
        self.items.get(to_skip).copied()
    }

    /// Holds the provided entity, if possible.
    pub fn hold(
        holding_entity: Entity,
        to_hold: Entity,
        world: &mut World,
    ) -> Result<(), HoldError> {
        let num_hands_required = match get_hands_to_hold(to_hold, world) {
            Some(hands) => hands,
            None => return Err(HoldError::CannotBeHeld),
        };

        if let Some(held_items) = world.get::<HeldItems>(holding_entity) {
            if held_items.items.contains(&to_hold) {
                return Err(HoldError::AlreadyHeld);
            }

            let num_hands_used: u8 = held_items.get_num_hands_used(world);
            if num_hands_used + num_hands_required.get() > held_items.hands {
                return Err(HoldError::NotEnoughHands);
            }
        } else {
            return Err(HoldError::CannotHold);
        }

        if let Some(mut held_items) = world.get_mut::<HeldItems>(holding_entity) {
            held_items.items.push(to_hold);
        }
        Ok(())
    }

    /// Stops holding the provided entity, if possible.
    pub fn unhold(
        holding_entity: Entity,
        to_unhold: Entity,
        world: &mut World,
    ) -> Result<(), UnholdError> {
        if let Some(mut held_items) = world.get_mut::<HeldItems>(holding_entity) {
            if let Some(index) = held_items.items.iter().position(|item| *item == to_unhold) {
                held_items.items.remove(index);
                return Ok(());
            }
        }

        Err(UnholdError::NotHolding)
    }
}

/// Describes the items being held by an entity.
#[derive(Debug)]
struct HeldItemsAttributeDescriber;

impl AttributeDescriber for HeldItemsAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let mut descriptions = Vec::new();
        if let Some(held_items) = world.get::<HeldItems>(entity) {
            let held_entity_names = held_items
                .items
                .iter()
                .map(|e| get_article_reference_name(*e, world))
                .collect::<Vec<String>>();

            for name in held_entity_names {
                descriptions.push(AttributeDescription::holds(name))
            }
        }

        descriptions
    }
}

impl DescribeAttributes for HeldItems {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(HeldItemsAttributeDescriber)
    }
}

/// Un-holds an item before it's moved out of the holding entity's inventory
pub fn unhold_on_put(
    notification: &Notification<BeforeActionNotification, PutAction>,
    world: &mut World,
) {
    let item = notification.contents.item;
    if let Some(holding_entity) = find_holding_entity(item, world) {
        match HeldItems::unhold(holding_entity, item, world) {
            Ok(_) => (),
            Err(UnholdError::NotHolding) => {
                panic!("{item:?} has holding entity but can't be un-held")
            }
        }
    }
}

/// Un-holds an item before the holding entity wears it
pub fn unhold_on_wear(
    notification: &Notification<BeforeActionNotification, WearAction>,
    world: &mut World,
) {
    let item = notification.contents.target;
    if let Some(holding_entity) = find_holding_entity(item, world) {
        match HeldItems::unhold(holding_entity, item, world) {
            Ok(_) => (),
            Err(UnholdError::NotHolding) => {
                panic!("{item:?} has holding entity but can't be un-held")
            }
        }
    }
}
