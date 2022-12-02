use bevy_ecs::prelude::*;

use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::{
    action::{Action, ActionResult},
    command::CommandParser,
    Direction, World,
};

use super::description::Description;

//TODO move locking and unlocking stuff to a LockedState component and remove this whole file

const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref OPEN_PATTERN: Regex = Regex::new("^open (the )?(?P<name>.*)").unwrap();
    static ref CLOSE_PATTERN: Regex = Regex::new("^close (the )?(?P<name>.*)").unwrap();
    static ref LOCK_PATTERN: Regex = Regex::new("^lock (the )?(?P<name>.*)").unwrap();
    static ref UNLOCK_PATTERN: Regex = Regex::new("^unlock (the )?(?P<name>.*)").unwrap();
}

#[derive(Bundle)]
pub struct DoorBundle {
    pub description: Description,
    pub door: Door,
    pub command_parser: CommandParser,
}

impl DoorBundle {
    pub fn new(description: Description, door: Door) -> DoorBundle {
        DoorBundle {
            description,
            door,
            command_parser: CommandParser {
                parse_fns: vec![Door::parse_command],
            },
        }
    }
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

    fn parse_command(
        door_id: Entity,
        input: &str,
        commanding_entity_id: Entity,
        world: &World,
    ) -> Option<Box<dyn Action>> {
        debug!("Door {door_id:?} parsing command {input:?} from {commanding_entity_id:?}");

        let desc = world
            .get::<Description>(door_id)
            .expect("Door should have a description");

        // opening
        if let Some(captures) = OPEN_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                if desc.matches(target_match.as_str()) {
                    let action = OpenDoorAction { door_id };
                    return Some(Box::new(action));
                }
            }
        }

        // closing
        if let Some(captures) = CLOSE_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                if desc.matches(target_match.as_str()) {
                    let action = CloseDoorAction { door_id };
                    return Some(Box::new(action));
                }
            }
        }

        //TODO locking and unlocking

        None
    }

    fn set_open(&mut self, new_open: bool, world: &mut World) {
        if self.is_open == new_open {
            return;
        }

        self.is_open = new_open;

        let mut other_side = world
            .get_mut::<Door>(self.other_side_id)
            .expect("Other side of door should be a door");
        other_side.is_open = new_open;
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

#[derive(Debug)]
struct OpenDoorAction {
    door_id: Entity,
}

impl Action for OpenDoorAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let door = world
            .get::<Door>(self.door_id)
            .expect("Entity to open should be a door");

        if door.is_open {
            return ActionResult::message(performing_entity, "It's already open.".to_string());
        }

        // if door is locked and can be unlocked, unlock it first
        //TODO

        // open that door
        //TODO

        ActionResult::message(
            performing_entity,
            "You fiddle with the door for a bit until you realize it's impossible to open."
                .to_string(),
        ) //TODO
    }
}

#[derive(Debug)]
struct CloseDoorAction {
    door_id: Entity,
}

impl Action for CloseDoorAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let mut door = world
            .get_mut::<Door>(self.door_id)
            .expect("Entity to close should be a door");

        if !door.is_open {
            return ActionResult::message(performing_entity, "It's already closed.".to_string());
        }

        door.is_open = false;
        let other_side_id = door.other_side_id;

        let mut other_side = world
            .get_mut::<Door>(other_side_id)
            .expect("Other side of door should be a door");
        other_side.is_open = false;

        //TODO send messages to entities on the other side of the door telling them the door closed

        let name = world
            .get::<Description>(self.door_id)
            .map_or("door", |n| &n.name);
        ActionResult::message(performing_entity, format!("You close the {name}."))
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
