use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;

use crate::{Aliases, Name, World};

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
}

#[derive(PartialEq, Eq, Debug, Component)]
pub struct Room {
    name: String,
    description: String,
    pub entities: HashSet<Entity>,
    connections: HashMap<Direction, Connection>,
}

impl Room {
    /// Creates a new empty room not connected to anywhere.
    pub fn new(name: String, description: String) -> Room {
        Room {
            name,
            description,
            entities: HashSet::new(),
            connections: HashMap::new(),
        }
    }

    /// Retrieves the connection in the provided direction, if there is one.
    pub fn connection_in_direction(&self, dir: &Direction) -> Option<&Connection> {
        self.connections.get(dir)
    }

    /// Adds a connection in the provided direction, overwriting any existing connection in that direction.
    pub fn connect(&mut self, dir: Direction, connection: Connection) {
        self.connections.insert(dir, connection);
    }

    /// Finds the entity with the provided name, if it exists in this room.
    pub fn find_entity_by_name(&self, name: &str, world: &World) -> Option<Entity> {
        for entity_id in &self.entities {
            if let Some(entity_name) = world.get::<Name>(*entity_id) {
                if entity_name.0.eq_ignore_ascii_case(name) {
                    return Some(*entity_id);
                }
            }

            if let Some(aliases) = world.get::<Aliases>(*entity_id) {
                if aliases.0.contains(name) {
                    return Some(*entity_id);
                }
            }
        }

        None
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Connection {
    pub destination_entity_id: Entity,
    pub connecting_entity_id: Option<Entity>,
}

impl Connection {
    /// Creates a new open connection to the provided room
    pub fn new_open(destination_entity_id: Entity) -> Connection {
        Connection {
            destination_entity_id,
            connecting_entity_id: None,
        }
    }

    /// Creates a new connection to the provided room that requires passing through the provided entity
    pub fn new_via_entity(
        destination_entity_id: Entity,
        connecting_entity_id: Entity,
    ) -> Connection {
        Connection {
            destination_entity_id,
            connecting_entity_id: Some(connecting_entity_id),
        }
    }
}

#[derive(Debug)]
pub struct RoomDescription {
    pub name: String,
    pub description: String,
    pub exits: Vec<ExitDescription>,
}

impl RoomDescription {
    /// Creates a `RoomDescription` for the provided room
    pub fn from_room(room: &Room, world: &World) -> RoomDescription {
        RoomDescription {
            name: room.name.clone(),
            description: room.description.clone(),
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
        room.connections
            .iter()
            .map(|(dir, connection)| {
                let destination_room = world
                    .get::<Room>(connection.destination_entity_id)
                    .expect("Destination entity should be a room");
                ExitDescription {
                    direction: *dir,
                    description: destination_room.name.clone(),
                }
            })
            .collect()
    }
}
