use std::{array, iter::once};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;

use crate::{
    color::Color,
    component::{AttributeDescription, Connection, Description, Location, Room},
    game_map::{Coordinates, GameMap, MapChar, MapIcon},
    input_parser::find_parsers_relevant_for,
    Direction,
};

const PLAYER_MAP_CHAR: MapChar = MapChar {
    bg_color: Color::Black,
    fg_color: Color::Green,
    value: '@',
};

lazy_static! {
    static ref BLANK_ICON: MapIcon =
        MapIcon::new_uniform(Color::Black, Color::DarkGray, ['.', '.', '.']);
}

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug, Clone)]
pub enum GameMessage {
    Room(RoomDescription),
    Entity(EntityDescription),
    DetailedEntity(DetailedEntityDescription),
    Help(HelpMessage),
    Message(String, MessageDelay),
    Error(String),
}

/// The amount of time to wait before any additional messages are displayed.
#[derive(Debug, Clone)]
pub enum MessageDelay {
    /// No time should be waited.
    None,
    /// A short amount of time should be waited.
    Short,
    /// A long amount of time should be waited.
    Long,
}

/// The description of an entity.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ActionDescription {
    pub format: String,
}

/// The description of an entity as part of a room description.
#[derive(Debug, Clone)]
pub enum RoomEntityDescription {
    Object(RoomObjectDescription),
    Living(RoomLivingEntityDescription),
    Connection(RoomConnectionEntityDescription),
}

/// A description of an object as part of a room description.
#[derive(Debug, Clone)]
pub struct RoomObjectDescription {
    /// The name of the entity.
    pub name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
}

/// A description of a living thing as part of a room description.
#[derive(Debug, Clone)]
pub struct RoomLivingEntityDescription {
    /// The name of the entity.
    pub name: String,
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

#[derive(Debug, Clone)]
pub struct RoomDescription {
    pub name: String,
    pub description: String,
    pub entities: Vec<RoomEntityDescription>,
    pub exits: Vec<ExitDescription>,
    pub map: Box<MapDescription<5>>,
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
            map: Box::new(MapDescription::for_entity(pov_entity, world)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

impl ExitDescription {
    /// Creates a list of exit descriptions for the provided room
    pub fn from_room(room: &Room, world: &World) -> Vec<ExitDescription> {
        // TODO order connections consistently
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

/// A collection of tiles around an entity.
/// `S` is the length and width of the map, in tiles.
#[derive(Debug, Clone)]
pub struct MapDescription<const S: usize> {
    /// The tiles in the map. Formatted as an array of rows.
    pub tiles: [[MapIcon; S]; S],
}

impl<const S: usize> MapDescription<S> {
    /// Creates a map centered on the location of the provided entity.
    fn for_entity(pov_entity: Entity, world: &World) -> MapDescription<S> {
        let center_coords = find_coordinates_of_entity(pov_entity, world);
        let center_index = S / 2;

        let tiles = array::from_fn(|row_index| {
            array::from_fn(|col_index| {
                if row_index == center_index && col_index == center_index {
                    let mut icon = icon_for_coords(center_coords, world);
                    icon.replace_center_char(PLAYER_MAP_CHAR);
                    return icon;
                }

                let x = center_coords.x + (col_index as i64 - center_index as i64);
                let y = center_coords.y - (row_index as i64 - center_index as i64);
                let z = center_coords.z;
                let parent = center_coords.parent.clone();

                icon_for_coords(&Coordinates { x, y, z, parent }, world)
            })
        });

        MapDescription { tiles }
    }
}

/// Finds the coordinates of the location the provided entity is in.
///
/// Panics if the entity does not have a location with coordinates.
fn find_coordinates_of_entity(entity: Entity, world: &World) -> &Coordinates {
    let location = world
        .get::<Location>(entity)
        .expect("entity should have a location");

    world
        .get::<Coordinates>(location.id)
        .expect("entity should be located in an entity with coordinates")
}

/// Finds the icon associated with the room at the provided location.
///
/// Panics if the provided coordinates map to an entity that isn't a room.
fn icon_for_coords(coords: &Coordinates, world: &World) -> MapIcon {
    if let Some(entity) = world.resource::<GameMap>().locations.get(coords) {
        return world
            .get::<Room>(*entity)
            .expect("coordinates should map to a room")
            .map_icon
            .clone();
    }

    BLANK_ICON.clone()
}

#[derive(Debug, Clone)]
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
