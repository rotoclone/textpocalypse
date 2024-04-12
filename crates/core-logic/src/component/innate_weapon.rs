use bevy_ecs::prelude::*;

use crate::{Container, Weapon};

/// Marks an entity as the weapon an entity uses when it has nothing equipped.
#[derive(Component)]
pub struct InnateWeapon;

impl InnateWeapon {
    /// Gets the innate weapon of the provided entity, if it has one.
    pub fn get(entity: Entity, world: &World) -> Option<(&Weapon, Entity)> {
        if let Some(inventory) = world.get::<Container>(entity) {
            if let Some(innate_weapon_entity) = inventory
                .get_entities_including_invisible()
                .iter()
                .find(|item| world.get::<InnateWeapon>(**item).is_some())
            {
                if let Some(innate_weapon) = world.get::<Weapon>(*innate_weapon_entity) {
                    return Some((innate_weapon, *innate_weapon_entity));
                }
            }
        }

        None
    }
}
