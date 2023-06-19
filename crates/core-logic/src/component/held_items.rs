use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::component::Item;

/// The things an entity is holding.
#[derive(Component)]
pub struct HeldItems {
    /// The number of hands the entity can hold things in.
    hands: u8,
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
    /// The entity's hands are already all holding items.
    HandsFull,
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
            let num_hands_used: u8 = held_items
                .items
                .iter()
                .map(|item| get_hands_to_hold(*item, world))
                .sum();
            //TODO
        } else {
            return Err(HoldError::CannotHold);
        }

        Ok(())
    }

    /// Stops holding the provided entity, if possible.
    pub fn unhold(
        holding_entity: Entity,
        to_unhold: Entity,
        world: &mut World,
    ) -> Result<(), UnholdError> {
        todo!() //TODO
    }
}

/// Gets the number of hands needed to hold the provided entity, if it's an item.
fn get_hands_to_hold(entity: Entity, world: &World) -> Option<NonZeroU8> {
    world.get::<Item>(entity).map(|item| item.hands_to_hold)
}
