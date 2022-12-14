use std::{array, iter::once};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use lazy_static::lazy_static;

use crate::{
    color::Color,
    component::{
        AttributeDescription, AttributeDetailLevel, Connection, Container, Description, Location,
        Room, Vitals, Volume, Weight,
    },
    game_map::{Coordinates, GameMap, MapChar, MapIcon},
    get_volume, get_weight,
    input_parser::find_parsers_relevant_for,
    value_change::ValueType,
    ConstrainedValue, Direction,
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
    Container(ContainerDescription),
    Vitals(VitalsDescription),
    ValueChange(ValueChangeDescription, MessageDelay),
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
    /// Creates an entity description for an entity from the perspective of another entity.
    pub fn for_entity(
        pov_entity: Entity,
        entity: Entity,
        desc: &Description,
        world: &World,
    ) -> EntityDescription {
        EntityDescription::for_entity_with_detail_level(
            pov_entity,
            entity,
            desc,
            AttributeDetailLevel::Basic,
            world,
        )
    }

    /// Creates an entity description for `entity`, with attribute descriptions of the provided detail level.
    fn for_entity_with_detail_level(
        pov_entity: Entity,
        entity: Entity,
        desc: &Description,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> EntityDescription {
        EntityDescription {
            name: desc.name.clone(),
            aliases: build_aliases(desc),
            article: desc.article.clone(),
            description: desc.description.clone(),
            attributes: desc
                .attribute_describers
                .iter()
                .flat_map(|d| d.describe(pov_entity, entity, detail_level, world))
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
            basic_desc: EntityDescription::for_entity_with_detail_level(
                looking_entity,
                entity,
                desc,
                AttributeDetailLevel::Advanced,
                world,
            ),
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

/// The description of a container.
#[derive(Debug, Clone)]
pub struct ContainerDescription {
    /// Descriptions of the items in the container.
    pub items: Vec<ContainerEntityDescription>,
    /// The total volume used by items in this container.
    pub used_volume: Volume,
    /// The maximum volume of items this container can hold, if it is limited.
    pub max_volume: Option<Volume>,
    /// The total weight used by items in this container.
    pub used_weight: Weight,
    /// The maximum weight of items this container can hold, if it is limited.
    pub max_weight: Option<Weight>,
}

impl ContainerDescription {
    /// Creates a container description for the provided container.
    pub fn from_container(container: &Container, world: &World) -> ContainerDescription {
        let items = container
            .entities
            .iter()
            .flat_map(|entity| ContainerEntityDescription::from_entity(*entity, world))
            .collect::<Vec<ContainerEntityDescription>>();

        let used_volume = items.iter().map(|item| item.volume).sum();
        let used_weight = items.iter().map(|item| item.weight).sum();

        ContainerDescription {
            items,
            used_volume,
            max_volume: container.volume,
            used_weight,
            max_weight: container.max_weight,
        }
    }
}

/// The description of an item in a container.
#[derive(Debug, Clone)]
pub struct ContainerEntityDescription {
    /// The name of the item.
    pub name: String,
    /// The volume of the item.
    pub volume: Volume,
    /// The weight of the item.
    pub weight: Weight,
}

impl ContainerEntityDescription {
    /// Creates a container entity description for the provided entity.
    /// Returns `None` if the provided entity has no `Description` component.
    pub fn from_entity(entity: Entity, world: &World) -> Option<ContainerEntityDescription> {
        let entity_ref = world.entity(entity);
        let desc = entity_ref.get::<Description>()?;
        let volume = get_volume(entity, world);
        let weight = get_weight(entity, world);

        Some(ContainerEntityDescription {
            name: desc.name.clone(),
            volume,
            weight,
        })
    }
}

/// The description of an entity's vitals.
#[derive(Debug, Clone)]
pub struct VitalsDescription {
    /// The health of the entity.
    pub health: ConstrainedValue<f32>,
    /// The non-hunger of the entity.
    pub satiety: ConstrainedValue<f32>,
    /// The non-thirst of the entity.
    pub hydration: ConstrainedValue<f32>,
    /// The non-tiredness of the entity.
    pub energy: ConstrainedValue<f32>,
}

impl VitalsDescription {
    /// Creates a vitals description for the provided vitals.
    pub fn from_vitals(vitals: &Vitals) -> VitalsDescription {
        VitalsDescription {
            health: vitals.health.clone(),
            satiety: vitals.satiety.clone(),
            hydration: vitals.hydration.clone(),
            energy: vitals.energy.clone(),
        }
    }
}

/// A description of a change of a single value.
#[derive(Debug, Clone)]
pub struct ValueChangeDescription {
    /// The message to include with the display of the new value.
    pub message: String,
    /// The type of value that changed.
    pub value_type: ValueType,
    /// The old value.
    pub old_value: ConstrainedValue<f32>,
    /// The new value.
    pub new_value: ConstrainedValue<f32>,
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
                    plural_name: desc.plural_name.clone(),
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

#[derive(Debug, Clone)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
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

/// A collection of tiles around an entity.
/// `S` is the length and width of the map, in tiles.
#[derive(Debug, Clone)]
pub struct MapDescription<const S: usize> {
    /// The tiles in the map. Formatted as an array of rows.
    pub tiles: [[MapIcon; S]; S],
}

impl<const S: usize> MapDescription<S> {
    /// Creates a map centered on the location of the provided entity.
    fn for_entity(
        pov_entity: Entity,
        center_coords: &Coordinates,
        world: &World,
    ) -> MapDescription<S> {
        let pov_coords = find_coordinates_of_entity(pov_entity, world);
        let center_index = S / 2;

        let tiles = array::from_fn(|row_index| {
            array::from_fn(|col_index| {
                let x = center_coords.x + (col_index as i64 - center_index as i64);
                let y = center_coords.y - (row_index as i64 - center_index as i64);
                let z = center_coords.z;
                let parent = center_coords.parent.clone();

                let current_coords = Coordinates { x, y, z, parent };

                let mut icon = icon_for_coords(&current_coords, world);

                if current_coords == *pov_coords {
                    icon.replace_center_char(PLAYER_MAP_CHAR);
                }

                icon
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
