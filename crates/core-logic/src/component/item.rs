use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

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

/// Gets the number of hands needed to hold the provided entity, if it's an item.
pub fn get_hands_to_hold(entity: Entity, world: &World) -> Option<NonZeroU8> {
    world.get::<Item>(entity).map(|item| item.hands_to_hold)
}

/// Describes the number of hands needed to hold an entity.
#[derive(Debug)]
struct HandsNeededAttributeDescriber;

impl AttributeDescriber for HandsNeededAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if detail_level >= AttributeDetailLevel::Advanced {
            if let Some(item) = world.get::<Item>(entity) {
                let hand_or_hands = if item.hands_to_hold.get() > 1 {
                    "hands"
                } else {
                    "hand"
                };

                return vec![AttributeDescription::does(format!(
                    "requires {} {} to hold",
                    item.hands_to_hold.get(),
                    hand_or_hands,
                ))];
            }
        }

        Vec::new()
    }
}

impl DescribeAttributes for Item {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(HandsNeededAttributeDescriber)
    }
}
