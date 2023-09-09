use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// Marks an entity as able to be picked up.
#[derive(Component)]
pub struct Item {
    /// The number of hands needed to equip the item.
    pub hands_to_equip: NonZeroU8,
}

impl Item {
    /// Creates an `Item` that requires the provided number of hands to equip.
    pub fn new(hands_to_equip: NonZeroU8) -> Item {
        Item { hands_to_equip }
    }

    /// Creates an `Item` that requires one hand to equip.
    pub fn new_one_handed() -> Item {
        Item {
            hands_to_equip: NonZeroU8::new(1).expect("1 should not be zero"),
        }
    }

    /// Creates an `Item` that requires two hands to equip.
    pub fn new_two_handed() -> Item {
        Item {
            hands_to_equip: NonZeroU8::new(2).expect("2 should not be zero"),
        }
    }
}

/// Gets the number of hands needed to equip the provided entity, if it's an item.
pub fn get_hands_to_equip(entity: Entity, world: &World) -> Option<NonZeroU8> {
    world.get::<Item>(entity).map(|item| item.hands_to_equip)
}

/// Describes the number of hands needed to equip an entity.
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
                let hand_or_hands = if item.hands_to_equip.get() > 1 {
                    "hands"
                } else {
                    "hand"
                };

                return vec![AttributeDescription::does(format!(
                    "requires {} {} to equip",
                    item.hands_to_equip.get(),
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
