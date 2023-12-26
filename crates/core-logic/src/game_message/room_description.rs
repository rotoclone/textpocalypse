use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::{Connection, Container, Description, Room},
    game_map::Coordinates,
    is_living_entity, Direction, MapDescription,
};

#[derive(Debug, Clone)]
pub struct RoomDescription {
    pub name: String,
    pub description: String,
    pub entities: Vec<RoomEntityDescription>,
    pub exits: Vec<ExitDescription>,
    pub map: Box<MapDescription<5>>,
}

/// The description of an entity as part of a room description.
#[derive(Debug, Clone)]
pub enum RoomEntityDescription {
    Object(RoomObjectDescription),
    Living(RoomLivingEntityDescription),
    Connection(RoomConnectionEntityDescription),
}

/// A description of an object as part of a room description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoomObjectDescription {
    /// The name of the entity.
    pub name: String,
    /// The plural name of the entity.
    pub plural_name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
}

/// A description of a living thing as part of a room description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoomLivingEntityDescription {
    /// The name of the entity.
    pub name: String,
    /// The plural name of the entity.
    pub plural_name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
}

/// A description of a connection to another room as part of a room description.
#[derive(Debug, Clone)]
pub struct RoomConnectionEntityDescription {
    /// The name of the entity.
    pub name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The direction the connection is in.
    pub direction: Direction,
}

#[derive(Debug, Clone)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

impl RoomDescription {
    /// Creates a `RoomDescription` for the provided room from the perspective of the provided entity.
    ///
    /// The provided Room, Container, and Coordinates should be on the same entity.
    pub fn from_room(
        room: &Room,
        container: &Container,
        coordinates: &Coordinates,
        pov_entity: Entity,
        world: &World,
    ) -> RoomDescription {
        let entity_descriptions = container
            .entities
            .iter()
            .filter(|entity| **entity != pov_entity)
            .filter_map(|entity| RoomEntityDescription::from_entity(*entity, world))
            .collect();

        RoomDescription {
            name: room.name.clone(),
            description: room.description.clone(),
            entities: entity_descriptions,
            exits: ExitDescription::from_container(container, world),
            map: Box::new(MapDescription::for_entity(pov_entity, coordinates, world)),
        }
    }
}

impl RoomEntityDescription {
    /// Creates a room entity description for the provided entity.
    pub fn from_entity(entity: Entity, world: &World) -> Option<RoomEntityDescription> {
        if let Some(desc) = world.get::<Description>(entity) {
            if is_living_entity(entity, world) {
                Some(RoomEntityDescription::Living(RoomLivingEntityDescription {
                    name: desc.room_name.clone(),
                    plural_name: desc.plural_name.clone(),
                    article: desc.article.clone(),
                }))
            } else if let Some(connection) = world.get::<Connection>(entity) {
                Some(RoomEntityDescription::Connection(
                    RoomConnectionEntityDescription {
                        name: desc.room_name.clone(),
                        article: desc.article.clone(),
                        direction: connection.direction,
                    },
                ))
            } else {
                Some(RoomEntityDescription::Object(RoomObjectDescription {
                    name: desc.room_name.clone(),
                    plural_name: desc.plural_name.clone(),
                    article: desc.article.clone(),
                }))
            }
        } else {
            None
        }
    }
}

impl ExitDescription {
    /// Creates a list of exit descriptions for the provided container.
    pub fn from_container(container: &Container, world: &World) -> Vec<ExitDescription> {
        container
            .get_connections(world)
            .iter()
            .sorted_by(|a, b| a.1.direction.cmp(&b.1.direction))
            .map(|(_, connection)| {
                let destination_room = world
                    .get::<Room>(connection.destination)
                    .expect("Destination entity should be a room");
                ExitDescription {
                    direction: connection.direction,
                    description: destination_room.name.clone(),
                }
            })
            .collect()
    }
}
