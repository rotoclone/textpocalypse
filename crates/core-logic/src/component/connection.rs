use bevy_ecs::prelude::*;

/// Describes a connection an entity makes to another room.
#[derive(PartialEq, Eq, Debug, Component)]
pub struct Connection {
    /// The direction the connection is in.
    pub direction: Direction,
    /// The ID of the room the entity connects to.
    pub destination: Entity,
    /// The ID of the entity representing other side of the connection, if there is one.
    pub other_side: Option<Entity>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Up,
    Down,
}
