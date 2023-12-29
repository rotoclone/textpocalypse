use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::{
    action::{PutAction, WearAction},
    find_wielding_entity, get_article_reference_name,
    notification::Notification,
    AttributeDescription,
};

use super::{
    get_hands_to_equip, AttributeDescriber, AttributeDetailLevel, BeforeActionNotification,
    DescribeAttributes,
};

/// The things an entity has equipped.
#[derive(Component)]
pub struct EquippedItems {
    /// The number of hands the entity can equip things in.
    pub hands: u8,
    /// The items equipped.
    items: Vec<Entity>,
}

/// An error when trying to equip something.
#[derive(Debug)]
pub enum EquipError {
    /// The entity cannot equip things.
    CannotEquip,
    /// The item in question cannot be equipped.
    CannotBeEquipped,
    /// The entity already has the item equipped.
    AlreadyEquipped,
    /// The entity doesn't have enough free hands to equip the item.
    NotEnoughHands,
}

/// An error when trying to stop wielding something.
#[derive(Debug)]
pub enum UnequipError {
    /// The entity doesn't have the item equipped.
    NotEquipped,
}

impl EquippedItems {
    /// Creates a `EquippedItems` with the provided number of hands.
    pub fn new(hands: u8) -> EquippedItems {
        EquippedItems {
            hands,
            items: Vec::new(),
        }
    }

    /// Determines whether the provided entity is currently equipped.
    pub fn is_equipped(&self, entity: Entity) -> bool {
        self.items.contains(&entity)
    }

    /// Determines how many hands are currently wielding items.
    pub fn get_num_hands_used(&self, world: &World) -> u8 {
        self.items
            .iter()
            .flat_map(|item| get_hands_to_equip(*item, world).map(|h: NonZeroU8| h.get()))
            .sum()
    }

    /// Returns all the equipped items, ordered from least-recently to most-recently equipped.
    pub fn get_items(&self) -> &Vec<Entity> {
        &self.items
    }

    /// Returns the item that has been equipped the longest, if there is one, skipping the provided number of items.
    pub fn get_oldest_item(&self, to_skip: usize) -> Option<Entity> {
        self.items.get(to_skip).copied()
    }

    /// Equips the provided entity, if possible.
    pub fn equip(
        equipping_entity: Entity,
        to_equip: Entity,
        world: &mut World,
    ) -> Result<(), EquipError> {
        let num_hands_required = match get_hands_to_equip(to_equip, world) {
            Some(hands) => hands,
            None => return Err(EquipError::CannotBeEquipped),
        };

        if let Some(equipped_items) = world.get::<EquippedItems>(equipping_entity) {
            if equipped_items.items.contains(&to_equip) {
                return Err(EquipError::AlreadyEquipped);
            }

            let num_hands_used: u8 = equipped_items.get_num_hands_used(world);
            if num_hands_used + num_hands_required.get() > equipped_items.hands {
                return Err(EquipError::NotEnoughHands);
            }
        } else {
            return Err(EquipError::CannotEquip);
        }

        if let Some(mut equipped_items) = world.get_mut::<EquippedItems>(equipping_entity) {
            equipped_items.items.push(to_equip);
        }
        Ok(())
    }

    /// Stops wielding the provided entity, if possible.
    pub fn unequip(
        wielding_entity: Entity,
        to_unequip: Entity,
        world: &mut World,
    ) -> Result<(), UnequipError> {
        if let Some(mut equipped_items) = world.get_mut::<EquippedItems>(wielding_entity) {
            if let Some(index) = equipped_items
                .items
                .iter()
                .position(|item| *item == to_unequip)
            {
                equipped_items.items.remove(index);
                return Ok(());
            }
        }

        Err(UnequipError::NotEquipped)
    }
}

/// Describes the items equipped by an entity.
#[derive(Debug)]
struct EquippedItemsAttributeDescriber;

impl AttributeDescriber for EquippedItemsAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let mut descriptions = Vec::new();
        if let Some(equipped_items) = world.get::<EquippedItems>(entity) {
            let equipped_entity_names = equipped_items
                .items
                .iter()
                .map(|e| get_article_reference_name(*e, world))
                .collect::<Vec<String>>();

            for name in equipped_entity_names {
                descriptions.push(AttributeDescription::wields(name))
            }
        }

        descriptions
    }
}

impl DescribeAttributes for EquippedItems {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(EquippedItemsAttributeDescriber)
    }
}

/// Unequips an item before it's moved out of the wielding entity's inventory
pub fn unequip_on_put(
    notification: &Notification<BeforeActionNotification, PutAction>,
    world: &mut World,
) {
    let item = notification.contents.item;
    if let Some(wielding_entity) = find_wielding_entity(item, world) {
        match EquippedItems::unequip(wielding_entity, item, world) {
            Ok(_) => (),
            Err(UnequipError::NotEquipped) => {
                panic!("{item:?} has wielding entity but can't be unequipped")
            }
        }
    }
}

/// Unequips an item before the wielding entity wears it
pub fn unequip_on_wear(
    notification: &Notification<BeforeActionNotification, WearAction>,
    world: &mut World,
) {
    let item = notification.contents.target;
    if let Some(wielding_entity) = find_wielding_entity(item, world) {
        match EquippedItems::unequip(wielding_entity, item, world) {
            Ok(_) => (),
            Err(UnequipError::NotEquipped) => {
                panic!("{item:?} has wielding entity but can't be unequipped")
            }
        }
    }
}
