use bevy_ecs::prelude::*;

use crate::{AttributeDescription, Direction};

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

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
    fn describe(
        &self,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(connection) = world.get::<Connection>(entity) {
            return vec![AttributeDescription::does(format!(
                "leads {}",
                connection.direction
            ))];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Connection {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(ConnectionAttributeDescriber)
    }
}
