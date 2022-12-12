use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{component::Description, game_map::MapIcon, Direction, World};

use super::Connection;

#[derive(PartialEq, Eq, Debug, Component)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub map_icon: MapIcon,
    pub entities: HashSet<Entity>,
}

impl Room {
    /// Creates a new empty room not connected to anywhere.
    pub fn new(name: String, description: String, map_icon: MapIcon) -> Room {
        Room {
            name,
            description,
            map_icon,
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
