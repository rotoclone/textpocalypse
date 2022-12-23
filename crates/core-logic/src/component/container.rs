use std::{collections::HashSet, fmt::Display, iter::Sum};

use bevy_ecs::prelude::*;

use crate::Direction;

use super::{Connection, Description};

/// The volume of an entity.
#[derive(Debug, Clone, Component)]
pub struct Volume(pub f32);

impl Sum for Volume {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Volume(iter.map(|x| x.0).sum())
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The weight of an entity.
#[derive(Debug, Clone, Component)]
pub struct Weight(pub f32);

impl Sum for Weight {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Weight(iter.map(|x| x.0).sum())
    }
}

impl Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Entities contained within an entity.
#[derive(Component)]
pub struct Container {
    /// The contained entities.
    pub entities: HashSet<Entity>,
    /// The maximum volume of items this container can hold, if it is limited.
    pub volume: Option<Volume>,
    /// The maximum weight of items this container can hold, if it is limited.
    pub max_weight: Option<Weight>,
}

impl Container {
    /// Creates an empty container that can hold an infinite amount of objects.
    pub fn new_infinite() -> Container {
        Container {
            entities: HashSet::new(),
            volume: None,
            max_weight: None,
        }
    }

    /// Creates an empty container.
    pub fn new(volume: Option<Volume>, max_weight: Option<Weight>) -> Container {
        Container {
            entities: HashSet::new(),
            volume,
            max_weight,
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

    /// Retrieves all the connections in this container.
    pub fn get_connections<'w>(&self, world: &'w World) -> Vec<(Entity, &'w Connection)> {
        self.entities
            .iter()
            .filter_map(|entity| world.get::<Connection>(*entity).map(|c| (*entity, c)))
            .collect()
    }

    /// Finds the entity with the provided name, if it exists in this container.
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
