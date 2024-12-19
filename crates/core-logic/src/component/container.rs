use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{
    action::PutAction,
    find_wearing_entity,
    notification::{Notification, VerifyResult},
    AttributeDescription, ContainerDescription, Direction, GameMessage, Invisible,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, Connection, DescribeAttributes, Description,
    OpenState, VerifyActionNotification, Volume, Weight,
};

/// Entities contained within an entity.
#[derive(Component)]
pub struct Container {
    /// The contained entities.
    entities: HashSet<Entity>,
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

    /// Gets all the entities in this container, from the perspective of the provided entity.
    pub fn get_entities(&self, pov_entity: Entity, world: &World) -> HashSet<Entity> {
        self.entities
            .iter()
            .copied()
            .filter(|entity| Invisible::is_visible_to(*entity, pov_entity, world))
            .collect()
    }

    /// Gets all the entities in this container.
    pub fn get_entities_including_invisible(&self) -> &HashSet<Entity> {
        &self.entities
    }

    /// Gets all the entities in this container mutably.
    pub fn get_entities_including_invisible_mut(&mut self) -> &mut HashSet<Entity> {
        &mut self.entities
    }

    /// Retrieves the entity that connects to the provided direction, if there is one.
    pub fn get_connection_in_direction<'w>(
        &self,
        dir: &Direction,
        pov_entity: Entity,
        world: &'w World,
    ) -> Option<(Entity, &'w Connection)> {
        self.get_connections(pov_entity, world)
            .into_iter()
            .find(|(_, connection)| connection.direction == *dir)
    }

    /// Retrieves all the connections in this container.
    pub fn get_connections<'w>(
        &self,
        pov_entity: Entity,
        world: &'w World,
    ) -> Vec<(Entity, &'w Connection)> {
        self.get_entities(pov_entity, world)
            .iter()
            .filter_map(|entity| world.get::<Connection>(*entity).map(|c| (*entity, c)))
            .collect()
    }

    /// Finds the entity with the provided name, if it exists in this container from the perspective of the provided entity.
    pub fn find_entity_by_name(
        &self,
        entity_name: &str,
        pov_entity: Entity,
        world: &World,
    ) -> Option<Entity> {
        for entity_id in &self.get_entities(pov_entity, world) {
            if let Some(desc) = world.get::<Description>(*entity_id) {
                if desc.matches(entity_name) {
                    return Some(*entity_id);
                }
            }
        }

        None
    }

    /// Determines if the provided entity is inside this container, or inside any container in this container, etc.
    pub fn contains_recursive(&self, entity: Entity, pov_entity: Entity, world: &World) -> bool {
        !self
            .find_recursive(|e| e == entity, pov_entity, world)
            .is_empty()
    }

    /// Determines if the provided entity is inside this container, or inside any container in this container, etc.
    ///
    /// Invisible entities will also be checked.
    pub fn contains_recursive_including_invisible(&self, entity: Entity, world: &World) -> bool {
        !self
            .find_recursive_including_invisible(|e| e == entity, world)
            .is_empty()
    }

    /// Finds all entities in this container (or in any container in this container, etc.) for which the provided function returns true.
    pub fn find_recursive(
        &self,
        match_fn: impl Fn(Entity) -> bool,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<Entity> {
        self.find_recursive_internal(
            &match_fn,
            &|container| container.get_entities(pov_entity, world),
            world,
            &mut vec![],
        )
    }

    /// Finds all entities in this container (or in any container in this container, etc.) for which the provided function returns true.
    ///
    /// Invisible entities will also be checked.
    pub fn find_recursive_including_invisible(
        &self,
        match_fn: impl Fn(Entity) -> bool,
        world: &World,
    ) -> Vec<Entity> {
        self.find_recursive_internal(
            &match_fn,
            &|container| container.get_entities_including_invisible().clone(),
            world,
            &mut vec![],
        )
    }

    fn find_recursive_internal(
        &self,
        match_fn: impl Fn(Entity) -> bool + Clone,
        get_entities_fn: &impl Fn(&Container) -> HashSet<Entity>,
        world: &World,
        contained_entities: &mut Vec<Entity>,
    ) -> Vec<Entity> {
        let mut found_entities = Vec::new();

        for contained_entity in get_entities_fn(self) {
            if contained_entities.contains(&contained_entity) {
                panic!("{contained_entity:?} contains itself")
            }
            contained_entities.push(contained_entity);

            if match_fn(contained_entity) {
                found_entities.push(contained_entity);
            }

            if let Some(container) = world.get::<Container>(contained_entity) {
                found_entities.extend(container.find_recursive_internal(
                    match_fn.clone(),
                    get_entities_fn,
                    world,
                    contained_entities,
                ));
            }
        }

        found_entities
    }

    /// Determines the total weight of all the items in the container.
    pub fn used_weight(&self, world: &World) -> Weight {
        self.entities
            .iter()
            .map(|e| Weight::get(*e, world))
            .sum::<Weight>()
    }

    /// Determines the total volume used by all the items in the container.
    pub fn used_volume(&self, world: &World) -> Volume {
        self.entities
            .iter()
            // items being worn are considered to have 0 volume for purposes of inventory size limits
            .filter(|e| find_wearing_entity(**e, world).is_none())
            .map(|e| Volume::get(*e, world))
            .sum::<Volume>()
    }
}

/// Describes the contents of an entity.
#[derive(Debug)]
struct ContainerAttributeDescriber;

impl AttributeDescriber for ContainerAttributeDescriber {
    fn describe(
        &self,
        pov_entity: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(container) = world.get::<Container>(entity) {
            if let Some(open_state) = world.get::<OpenState>(entity) {
                if !open_state.is_open {
                    return Vec::new();
                }
            }

            let message = GameMessage::Container(ContainerDescription::from_container(
                container, pov_entity, world,
            ));
            return vec![AttributeDescription::Message(Box::new(message))];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Container {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(ContainerAttributeDescriber)
    }
}

/// Prevents containers from getting overfilled.
pub fn limit_container_contents(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.item;
    let destination = notification.contents.destination;
    let performing_entity = notification.notification_type.performing_entity;

    let container = world
        .get::<Container>(destination)
        .expect("destination entity should be a container");

    let item_weight = Weight::get(item, world);
    if let Some(max_weight) = &container.max_weight {
        let used_weight = container.used_weight(world);
        if used_weight + item_weight > *max_weight {
            let item_name = Description::get_reference_name(item, Some(performing_entity), world);
            let message = if destination == performing_entity {
                format!("{item_name} is too heavy for you to hold.")
            } else {
                let destination_name =
                    Description::get_reference_name(destination, Some(performing_entity), world);
                format!("{item_name} is too heavy for {destination_name}.")
            };
            return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
        }
    }

    let item_volume = Volume::get(item, world);
    if let Some(max_volume) = &container.volume {
        let used_volume = container.used_volume(world);
        if used_volume + item_volume > *max_volume {
            let item_name = Description::get_reference_name(item, Some(performing_entity), world);
            let message = if destination == performing_entity {
                format!("{item_name} is too big for you to hold.")
            } else {
                let destination_name =
                    Description::get_reference_name(destination, Some(performing_entity), world);
                format!("{item_name} won't fit in {destination_name}.")
            };
            return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
        }
    }

    VerifyResult::valid()
}
