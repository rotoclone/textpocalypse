use bevy_ecs::prelude::*;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{Direction, World};

//TODO move locking and unlocking stuff to a LockedState component and remove this whole file

lazy_static! {
    static ref LOCK_PATTERN: Regex = Regex::new("^lock (the )?(?P<name>.*)").unwrap();
    static ref UNLOCK_PATTERN: Regex = Regex::new("^unlock (the )?(?P<name>.*)").unwrap();
}

struct KeyId(u32);

//TODO somehow include information about whether the door is locked or unlocked or open or closed in its description
//TODO split this into generic "Openable", "Lockable", and "Connector" components?
#[derive(Component)]
pub struct Door {
    /// Whether the door is currently open.
    is_open: bool,
    /// Whether the door is currently locked.
    is_locked: bool,
    /// Whether the door can be locked without a key.
    is_lockable: bool,
    /// Whether the door can be unlocked without a key.
    is_unlockable: bool,
    /// The ID of the key needed to lock or unlock the door, if it has a lock.
    key_id: Option<KeyId>,
    /// The ID of the other side of the door.
    other_side_id: Entity,
    /// The direction this door is in.
    direction: Direction,
}

impl Door {
    pub fn new_closed_no_lock(direction: Direction, other_side_id: Entity) -> Door {
        Door {
            is_open: false,
            is_locked: false,
            is_lockable: false,
            is_unlockable: false,
            key_id: None,
            other_side_id,
            direction,
        }
    }

    fn set_locked(&mut self, new_locked: bool, world: &mut World) {
        if self.is_locked == new_locked {
            return;
        }

        self.is_locked = new_locked;
        let mut other_side = world
            .get_mut::<Door>(self.other_side_id)
            .expect("Other side of door should be a door");
        other_side.is_locked = new_locked;
    }

    fn can_unlock(&self, entity_id: Entity, world: &World) -> bool {
        if self.is_unlockable {
            return true;
        }

        /* TODO make this compile
        if let Some(key_id) = self.key_id {
            if let Some(inventory) = world.get::<Inventory>(entity_id) {
                for inventory_entity_id in inventory.entities.iter() {
                    if let Some(key) = world.get::<Key>(inventory_entity_id) {
                        return key.id == key_id;
                    }
                }
            }
        }
        */

        false
    }
}

/* TODO remove
impl ConnectingEntity for Door {
    fn can_pass(&self, _: EntityId, _: &World) -> bool {
        self.is_open
    }

    fn can_see_through(&self) -> bool {
        self.is_open
    }

    fn on_attempt_open(&mut self, entity_id: EntityId, world: &mut World) {
        if self.is_open {
            return;
        }

        if self.is_locked && (self.can_unlock(entity_id, world)) {
            self.set_locked(false, world);
            //TODO send message about unlocking the door
        }

        if !self.is_locked {
            self.set_open(true, world);
            //TODO send message about opening the door
        }
    }

    fn on_pass(&self, entity_id: EntityId, world: &World) {
        todo!() //TODO
    }

    fn on_fail_pass(&self, entity_id: EntityId, world: &World) {
        todo!() //TODO
    }
}
*/
