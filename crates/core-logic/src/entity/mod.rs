use crate::{Action, EntityId, LocationId, World};

mod connecting_entity;

pub trait Entity {
    /// Returns the name used to refer to this entity in commands.
    fn get_name(&self) -> &str;

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
