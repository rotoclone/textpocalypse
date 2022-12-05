use std::{collections::HashSet, iter::once};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use log::debug;

use crate::{input_parser::find_parsers_relevant_for, Direction};

use super::Connection;

/// The description of an entity.
#[derive(Component, Debug)]
pub struct Description {
    /// The name of the entity.
    pub name: String,
    /// The name to use when referring to the entity as part of a room description.
    pub room_name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The alternate names of the entity.
    pub aliases: HashSet<String>,
    /// The description of the entity.
    pub description: String,
}

impl Description {
    /// Determines whether the provided input refers to the entity with this description.
    pub fn matches(&self, input: &str) -> bool {
        debug!("Checking if {input:?} matches {self:?}");
        self.name.eq_ignore_ascii_case(input)
            || self.room_name.eq_ignore_ascii_case(input)
            || self.aliases.contains(input)
    }
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
}

impl EntityDescription {
    /// Creates an entity description from a `Description`.
    pub fn from_description(desc: &Description) -> EntityDescription {
        EntityDescription {
            name: desc.name.clone(),
            aliases: build_aliases(desc),
            article: desc.article.clone(),
            description: desc.description.clone(),
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
            basic_desc: EntityDescription::from_description(desc),
            actions: build_action_descriptions(looking_entity, entity, world),
        }
    }
}

fn build_action_descriptions(
    looking_entity: Entity,
    entity: Entity,
    world: &World,
) -> Vec<ActionDescription> {
    find_parsers_relevant_for(looking_entity, world)
        .flat_map(|p| p.input_formats_for(entity, world))
        .flatten()
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
