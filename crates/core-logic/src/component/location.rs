use bevy_ecs::prelude::*;

/// The location of an entity.
#[derive(Component)]
pub struct Location {
    /// The ID of the other entity the entity is located in.
    pub id: Entity,
}
