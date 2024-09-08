use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{
    AttributeDescriber, AttributeDetailLevel, AttributeSection, AttributeSectionName,
    DescribeAttributes, SectionAttributeDescription,
};

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
struct ItemAttributeDescriber;

impl AttributeDescriber for ItemAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(item) = world.get::<Item>(entity) {
            return vec![AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Item,
                attributes: vec![SectionAttributeDescription {
                    name: "Hands to equip".to_string(),
                    description: item.hands_to_equip.to_string(),
                }],
            })];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Item {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(ItemAttributeDescriber)
    }
}
