use std::fmt::Display;

use bevy_ecs::prelude::*;

use crate::{AttributeDescription, AttributeType};

use super::{AttributeDescriber, DescribeAttributes};

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

/// Describes where the entity connects to.
#[derive(Debug)]
struct ConnectionAttributeDescriber;

impl AttributeDescriber for ConnectionAttributeDescriber {
    fn describe(&self, entity: Entity, world: &World) -> Vec<AttributeDescription> {
        if let Some(connection) = world.get::<Connection>(entity) {
            return vec![AttributeDescription {
                attribute_type: AttributeType::Does,
                description: format!("leads {}", connection.direction),
            }];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Connection {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(ConnectionAttributeDescriber)
    }
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

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Direction::North => "north",
            Direction::NorthEast => "northeast",
            Direction::East => "east",
            Direction::SouthEast => "southeast",
            Direction::South => "south",
            Direction::SouthWest => "southwest",
            Direction::West => "west",
            Direction::NorthWest => "northwest",
            Direction::Up => "up",
            Direction::Down => "down",
        };

        write!(f, "{string}")
    }
}
