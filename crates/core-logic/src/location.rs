use std::collections::{HashMap, HashSet};

use crate::{EntityId, LocationId, World};

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

pub struct Location {
    name: String,
    description: String,
    pub entities: HashSet<EntityId>,
    connections: HashMap<Direction, Connection>,
}

impl Location {
    /// Creates a new empty location not connected to anywhere.
    pub fn new(name: String, description: String) -> Location {
        Location {
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
}

pub struct Connection {
    pub location_id: LocationId,
    pub connecting_entity_id: Option<EntityId>,
}

impl Connection {
    /// Creates a new open connection to the provided location
    pub fn new_open(location_id: LocationId) -> Connection {
        Connection {
            location_id,
            connecting_entity_id: None,
        }
    }

    /// Creates a new connection to the provided location that requires passing through the provided entity
    pub fn new_via_entity(location_id: LocationId, connecting_entity_id: EntityId) -> Connection {
        Connection {
            location_id,
            connecting_entity_id: Some(connecting_entity_id),
        }
    }
}

#[derive(Debug)]
pub struct LocationDescription {
    pub name: String,
    pub description: String,
    pub exits: Vec<ExitDescription>,
}

impl LocationDescription {
    /// Creates a `LocationDescription` for the provided location
    pub fn from_location(location: &Location, world: &World) -> LocationDescription {
        LocationDescription {
            name: location.name.clone(),
            description: location.description.clone(),
            exits: ExitDescription::from_location(location, world),
        }
    }
}

#[derive(Debug)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

impl ExitDescription {
    /// Creates a list of exit descriptions for the provided location
    pub fn from_location(location: &Location, world: &World) -> Vec<ExitDescription> {
        location
            .connections
            .iter()
            .map(|(dir, connection)| {
                let location = world.get_location(connection.location_id);
                ExitDescription {
                    direction: *dir,
                    description: location.name.clone(),
                }
            })
            .collect()
    }
}
