use bevy_ecs::prelude::*;

use crate::{
    game_map::Coordinates, AttributeDescription, Direction, GameMessage, InternalMessageCategory,
    MessageCategory, MessageDelay, RoomDescription,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, Container, DescribeAttributes, OpenState, Room,
};

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
        pov_entity: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let mut descriptions = Vec::new();

        if let Some(connection) = world.get::<Connection>(entity) {
            descriptions.push(AttributeDescription::does(format!(
                "leads {}",
                connection.direction
            )));

            let can_see_through = world
                .get::<OpenState>(entity)
                .map(|state| state.is_open)
                .unwrap_or(true);
            if can_see_through {
                let destination_room = world
                    .get::<Room>(connection.destination)
                    .expect("connection should lead to a room");
                let destination_container = world
                    .get::<Container>(connection.destination)
                    .expect("connecting room should be a container");
                let destination_coords = world
                    .get::<Coordinates>(connection.destination)
                    .expect("connecting room should have coordinates");
                descriptions.push(AttributeDescription::Message(GameMessage::Message {
                    content: "Through it, you see:".to_string(),
                    category: MessageCategory::Internal(InternalMessageCategory::Misc),
                    delay: MessageDelay::None,
                    decorations: Vec::new(),
                }));
                descriptions.push(AttributeDescription::Message(GameMessage::Room(
                    RoomDescription::from_room(
                        destination_room,
                        destination_container,
                        destination_coords,
                        pov_entity,
                        world,
                    ),
                )));
            }
        }

        descriptions
    }
}

impl DescribeAttributes for Connection {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(ConnectionAttributeDescriber)
    }
}
