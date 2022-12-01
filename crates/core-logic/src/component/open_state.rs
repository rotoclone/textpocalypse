use bevy_ecs::prelude::*;

/// Describes whether an entity is open or closed.
#[derive(Component)]
pub struct OpenState {
    /// Whether the entity is open.
    pub open: bool,
    /// The ID of the other side of the entity, if it is a connection.
    pub other_side: Option<Entity>,
}
