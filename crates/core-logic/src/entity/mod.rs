use std::collections::HashSet;

use crate::{Action, EntityId, LocationId, World};

mod connecting_entity;

pub trait Entity: Send + Sync {
    /// Returns the display name of this entity.
    fn get_name(&self) -> &str;

    /// Returns the set of aliases that can be used to refer to this entity in commands.
    fn get_aliases(&self) -> &HashSet<String>;

    /// Returns a basic description of this entity.
    fn get_description(&self) -> &str;

    /// Returns the ID of the location this entity is in.
    fn get_location_id(&self) -> LocationId;

    /// Sets the location this entity is in.
    fn set_location_id(&mut self, location_id: LocationId);

    /// Called when the game world ticks.
    fn on_tick(&mut self);

    /// Called when an entity submits a command in the presence of this entity. Returns any action that should be performed as a result of the command.
    fn on_command(
        &self,
        entity_id: EntityId,
        command: String,
        world: &World,
    ) -> Option<Box<dyn Action>>;
}

#[derive(Debug)]
pub struct EntityDescription {
    pub name: String,
    pub description: String,
}

impl EntityDescription {
    /// Creates an `EntityDescription` for the provided entity
    pub fn from_entity(entity: &dyn Entity) -> EntityDescription {
        EntityDescription {
            name: entity.get_name().to_string(),
            description: entity.get_description().to_string(),
        }
    }
}
