use std::num::NonZeroU8;

use bevy_ecs::prelude::*;

use crate::{
    action::{PutAction, WearAction},
    find_wielding_entity,
    notification::Notification,
    AttributeDescription, Description,
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

    /// Determines whether the provided entity has the provided item equipped.
    pub fn is_equipped(entity: Entity, item: Entity, world: &World) -> bool {
        world
            .get::<EquippedItems>(entity)
            .map(|equipped| equipped.contains(item))
            .unwrap_or(false)
    }

    /// Determines whether the provided entity is currently equipped.
    pub fn contains(&self, entity: Entity) -> bool {
        self.items.contains(&entity)
    }

    /// Determines how many hands are currently wielding items.
    pub fn get_num_hands_used(&self, world: &World) -> u8 {
        self.items
            .iter()
            .flat_map(|item| get_hands_to_equip(*item, world).map(|h: NonZeroU8| h.get()))
            .sum()
    }

    /// Determines how many hands are currently available to wield items.
    pub fn get_num_hands_free(&self, world: &World) -> u8 {
        self.hands.saturating_sub(self.get_num_hands_used(world))
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

    /// Determines which items the provided entity should unequip in order to have the provided number of hands free.
    /// If the entity already has at least the provided number of hands free, an empty list will be returned.
    pub fn get_items_to_unequip_to_free_hands(
        entity: Entity,
        free_hands_needed: u8,
        world: &World,
    ) -> Vec<Entity> {
        let mut items_to_unequip = Vec::new();
        if let Some(equipped_items) = world.get::<EquippedItems>(entity) {
            let num_hands_available = equipped_items.get_num_hands_free(world);
            if free_hands_needed > num_hands_available {
                // not enough free hands, figure out which items to unequip
                let num_hands_to_free = free_hands_needed - num_hands_available;
                let mut num_hands_freed = 0;
                let mut items_checked = 0;
                while num_hands_to_free > num_hands_freed {
                    if let Some(item) = equipped_items.get_oldest_item(items_checked) {
                        if let Some(hands_to_equip) = get_hands_to_equip(item, world) {
                            items_to_unequip.push(item);
                            num_hands_freed += hands_to_equip.get();
                        }
                        items_checked += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        items_to_unequip
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
                .map(|e| Description::get_article_reference_name(*e, world))
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
