use bevy_ecs::prelude::*;

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
        todo!() //TODO
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
