use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

/// Marks an entity as able to be picked up.
#[derive(Component)]
pub struct Item {
    /// The number of hands needed to hold the item.
    pub hands_to_hold: NonZeroU8,
}

impl Item {
    /// Creates an `Item` that requires the provided number of hands to hold.
    pub fn new(hands_to_hold: NonZeroU8) -> Item {
        Item { hands_to_hold }
    }

    /// Creates an `Item` that requires one hand to hold.
    pub fn new_one_handed() -> Item {
        Item {
            hands_to_hold: NonZeroU8::new(1).expect("1 should not be zero"),
        }
    }

    /// Creates an `Item` that requires two hands to hold.
    pub fn new_two_handed() -> Item {
        Item {
            hands_to_hold: NonZeroU8::new(2).expect("2 should not be zero"),
        }
    }
}
