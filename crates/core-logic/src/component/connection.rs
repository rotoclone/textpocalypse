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

impl Direction {
    /// Parses the provided string to a `Direction`. Returns `None` if the string doesn't map to any direction.
    pub fn parse(input: &str) -> Option<Direction> {
        match input {
            "n" | "north" => Some(Direction::North),
            "ne" | "northeast" => Some(Direction::NorthEast),
            "e" | "east" => Some(Direction::East),
            "se" | "southeast" => Some(Direction::SouthEast),
            "s" | "south" => Some(Direction::South),
            "sw" | "southwest" => Some(Direction::SouthWest),
            "w" | "west" => Some(Direction::West),
            "nw" | "northwest" => Some(Direction::NorthWest),
            "u" | "up" => Some(Direction::Up),
            "d" | "down" => Some(Direction::Down),
            _ => None,
        }
    }
}
