use std::iter::once;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::{AttributeDescription, Connection, Description, Direction, Room},
    input_parser::find_parsers_relevant_for,
};

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug)]
pub enum GameMessage {
    Room(RoomDescription),
    Entity(EntityDescription),
    DetailedEntity(DetailedEntityDescription),
    Help(HelpMessage),
    Message(String),
    Error(String),
}

/// The description of an entity.
#[derive(Debug)]
pub struct EntityDescription {
    /// The name of the entity.
    pub name: String,
    /// Other names for the entity.
    pub aliases: Vec<String>,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The description of the entity.
    pub description: String,
    /// Descriptions of dynamic attributes of the entity.
    pub attributes: Vec<AttributeDescription>,
}

impl EntityDescription {
    /// Creates an entity description for `entity`.
    pub fn for_entity(entity: Entity, desc: &Description, world: &World) -> EntityDescription {
        EntityDescription {
            name: desc.name.clone(),
            aliases: build_aliases(desc),
            article: desc.article.clone(),
            description: desc.description.clone(),
            attributes: desc
                .attribute_describers
                .iter()
                .flat_map(|d| d.describe(entity, world))
                .collect(),
        }
    }
}

fn build_aliases(desc: &Description) -> Vec<String> {
    once(desc.room_name.clone())
        .into_iter()
        .chain(desc.aliases.clone().into_iter())
        .filter(|name| name != &desc.name)
        .collect()
}

/// The detailed description of an entity.
#[derive(Debug)]
pub struct DetailedEntityDescription {
    pub basic_desc: EntityDescription,
    /// Descriptions of the actions that can be performed on the entity.
    pub actions: Vec<ActionDescription>,
}

impl DetailedEntityDescription {
    /// Creates a detailed entity description for `entity` being looked at by `looking_entity`.
    pub fn for_entity(
        looking_entity: Entity,
        entity: Entity,
        desc: &Description,
        world: &World,
    ) -> DetailedEntityDescription {
        DetailedEntityDescription {
            basic_desc: EntityDescription::for_entity(entity, desc, world),
            actions: build_action_descriptions_for_entity(looking_entity, entity, world),
        }
    }
}

/// Builds a list of descriptions of actions `looking_entity` can perform on `entity`.
fn build_action_descriptions_for_entity(
    looking_entity: Entity,
    entity: Entity,
    world: &World,
) -> Vec<ActionDescription> {
    find_parsers_relevant_for(looking_entity, world)
        .flat_map(|p| p.get_input_formats_for(entity, world))
        .flatten()
        .unique()
        .map(|format| ActionDescription { format })
        .collect()
}

/// Builds a list of descriptions of actions an entity can perform.
fn build_available_action_descriptions(
    looking_entity: Entity,
    world: &World,
) -> Vec<ActionDescription> {
    find_parsers_relevant_for(looking_entity, world)
        .flat_map(|p| p.get_input_formats())
        .unique()
        .map(|format| ActionDescription { format })
        .collect()
}

#[derive(Debug)]
pub struct ActionDescription {
    pub format: String,
}

/// The description of an entity as part of a room description.
#[derive(Debug)]
pub enum RoomEntityDescription {
    Object(RoomObjectDescription),
    Living(RoomLivingEntityDescription),
    Connection(RoomConnectionEntityDescription),
}

/// A description of an object as part of a room description.
#[derive(Debug)]
pub struct RoomObjectDescription {
    /// The name of the entity.
    pub name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
}

/// A description of a living thing as part of a room description.
#[derive(Debug)]
pub struct RoomLivingEntityDescription {
    /// The name of the entity.
    pub name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
}

/// A description of a connection to another room as part of a room description.
#[derive(Debug)]
pub struct RoomConnectionEntityDescription {
    /// The name of the entity.
    pub name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The direction the connection is in.
    pub direction: Direction,
}

impl RoomEntityDescription {
    /// Creates a room entity description for the provided entity.
    pub fn from_entity(entity: Entity, world: &World) -> Option<RoomEntityDescription> {
        if let Some(desc) = world.get::<Description>(entity) {
            if let Some(connection) = world.get::<Connection>(entity) {
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
                    article: desc.article.clone(),
                }))
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct RoomDescription {
    pub name: String,
    pub description: String,
    pub entities: Vec<RoomEntityDescription>,
    pub exits: Vec<ExitDescription>,
}

impl RoomDescription {
    /// Creates a `RoomDescription` for the provided room from the perspective of the provided entity.
    pub fn from_room(room: &Room, pov_entity: Entity, world: &World) -> RoomDescription {
        let entity_descriptions = room
            .entities
            .iter()
            .filter(|entity| **entity != pov_entity)
            .filter_map(|entity| RoomEntityDescription::from_entity(*entity, world))
            .collect();

        RoomDescription {
            name: room.name.clone(),
            description: room.description.clone(),
            entities: entity_descriptions,
            exits: ExitDescription::from_room(room, world),
        }
    }
}

#[derive(Debug)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

impl ExitDescription {
    /// Creates a list of exit descriptions for the provided room
    pub fn from_room(room: &Room, world: &World) -> Vec<ExitDescription> {
        room.get_connections(world)
            .iter()
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

#[derive(Debug)]
pub struct HelpMessage {
    /// Descriptions of the actions that can be performed.
    pub actions: Vec<ActionDescription>,
}

impl HelpMessage {
    /// Creates a help message for the provided entity.
    pub fn for_entity(entity: Entity, world: &World) -> HelpMessage {
        HelpMessage {
            actions: build_available_action_descriptions(entity, world),
        }
    }
}
