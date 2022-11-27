use lazy_static::lazy_static;
use regex::Regex;

use crate::{Action, EntityId, LocationId, World};

use super::Entity;

trait ConnectingEntity {
    /// Returns whether the provided entity can pass through this entity.
    fn can_pass(&self, entity_id: EntityId, world: &World) -> bool;

    /// Returns whether this entity can be seen through.
    fn can_see_through(&self) -> bool;

    /// Called when an entity attempts to open this.
    fn on_attempt_open(&mut self, entity_id: EntityId, world: &World);

    /// Called when an entity passes through this.
    fn on_pass(&self, entity_id: EntityId, world: &World);

    /// Called when an entity attempts to pass through this but cannot.
    fn on_fail_pass(&self, entity_id: EntityId, world: &World);
}

lazy_static! {
    static ref OPEN_PATTERN: Regex = Regex::new("^open (the )?(?P<name>.*)").unwrap();
    static ref CLOSE_PATTERN: Regex = Regex::new("^close (the )?(?P<name>.*)").unwrap();
    static ref SLAM_PATTERN: Regex = Regex::new("^slam (the )?(?P<name>.*)").unwrap();
}

struct Door {
    /// The name of the door.
    name: String,
    /// The ID of the location the door is in.
    location_id: LocationId,
    /// Whether the door is currently open.
    is_open: bool,
    /// Whether the door is currently locked.
    is_locked: bool,
    /// Whether the door can be locked without a key.
    is_lockable: bool,
    /// Whether the door can be unlocked without a key.
    is_unlockable: bool,
    /// The ID of the key needed to lock or unlock the door, if it has a lock.
    key_id: Option<EntityId>,
    /// The ID of the other side of the door.
    other_side_id: EntityId,
}

impl Entity for Door {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn set_location_id(&mut self, location_id: LocationId) {
        self.location_id = location_id;
    }

    fn on_tick(&mut self) {
        // do nothing
    }

    fn on_command(
        &self,
        entity_id: EntityId,
        command: String,
        world: &World,
    ) -> Option<Box<dyn Action>> {
        todo!() //TODO
    }
}

impl ConnectingEntity for Door {
    fn can_pass(&self, _: EntityId, _: &World) -> bool {
        self.is_open
    }

    fn can_see_through(&self) -> bool {
        false
    }

    fn on_attempt_open(&mut self, entity_id: EntityId, world: &World) {
        todo!() //TODO
    }

    fn on_pass(&self, entity_id: EntityId, world: &World) {
        todo!() //TODO
    }

    fn on_fail_pass(&self, entity_id: EntityId, world: &World) {
        todo!() //TODO
    }
}
