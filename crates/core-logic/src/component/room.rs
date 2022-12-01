use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{
    component::{Description, RoomEntityDescription},
    Direction, World,
};

use super::Connection;

#[derive(PartialEq, Eq, Debug, Component)]
pub struct Room {
    name: String,
    description: String,
    pub entities: HashSet<Entity>,
}

impl Room {
    /// Creates a new empty room not connected to anywhere.
    pub fn new(name: String, description: String) -> Room {
        Room {
            name,
            description,
            entities: HashSet::new(),
        }
    }

    /// Retrieves the entity that connects to the provided direction, if there is one.
    pub fn get_connection_in_direction<'w>(
        &self,
        dir: &Direction,
        world: &'w World,
    ) -> Option<(Entity, &'w Connection)> {
        self.get_connections(world)
            .into_iter()
            .find(|(_, connection)| connection.direction == *dir)
    }

    /// Retrieves all the connections in this room.
    pub fn get_connections<'w>(&self, world: &'w World) -> Vec<(Entity, &'w Connection)> {
        self.entities
            .iter()
            .filter_map(|entity| world.get::<Connection>(*entity).map(|c| (*entity, c)))
            .collect()
    }

    /// Finds the entity with the provided name, if it exists in this room.
    pub fn find_entity_by_name(&self, entity_name: &str, world: &World) -> Option<Entity> {
        for entity_id in &self.entities {
            if let Some(desc) = world.get::<Description>(*entity_id) {
                if desc.matches(entity_name) {
                    return Some(*entity_id);
                }
            }
        }

        None
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
