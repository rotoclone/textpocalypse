use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{
    action::PutAction,
    get_reference_name, get_weight,
    notification::{Notification, VerifyResult},
    AttributeDescription, ContainerDescription, Direction, GameMessage,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, Connection, DescribeAttributes, Description,
    VerifyActionNotification, Volume, Weight,
};

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

    /// Determines if the provided entity is inside this container, or inside any container in this container, etc.
    pub fn contains_recursive(&self, entity: Entity, world: &World) -> bool {
        self.contains_recursive_internal(entity, world, &mut vec![])
    }

    fn contains_recursive_internal(
        &self,
        entity: Entity,
        world: &World,
        contained_entities: &mut Vec<Entity>,
    ) -> bool {
        for contained_entity in &self.entities {
            if contained_entities.contains(contained_entity) {
                panic!("{contained_entity:?} contains itself")
            }
            contained_entities.push(*contained_entity);

            if entity == *contained_entity {
                return true;
            }

            if let Some(container) = world.get::<Container>(*contained_entity) {
                if container.contains_recursive_internal(entity, world, contained_entities) {
                    return true;
                }
            }
        }

        false
    }
}

/// Describes the contents of an entity.
#[derive(Debug)]
struct ContainerAttributeDescriber;

impl AttributeDescriber for ContainerAttributeDescriber {
    fn describe(
        &self,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        //TODO don't do this if the performing entity wouldn't be able to see the container's contents, like if they're looking at another player or into a closed item
        if let Some(container) = world.get::<Container>(entity) {
            let message =
                GameMessage::Container(ContainerDescription::from_container(container, world));
            return vec![AttributeDescription::Message(message)];
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
    let container = world
        .get::<Container>(notification.contents.destination)
        .expect("destination entity should be a container");

    let item_weight = get_weight(notification.contents.item, world);
    if let Some(max_weight) = &container.max_weight {
        let used_weight = container
            .entities
            .iter()
            .map(|e| get_weight(*e, world))
            .sum::<Weight>();
        if used_weight + item_weight > *max_weight {
            let item_name = get_reference_name(notification.contents.item, world);
            let message = format!("{item_name} is too heavy.");
            return VerifyResult::invalid(
                notification.notification_type.performing_entity,
                GameMessage::Error(message),
            );
        }
    }

    if let Some(item_volume) = world.get::<Volume>(notification.contents.item).cloned() {
        if let Some(max_volume) = &container.volume {
            let used_volume = container
                .entities
                .iter()
                .map(|e| world.get::<Volume>(*e).cloned().unwrap_or(Volume(0.0)))
                .sum::<Volume>();
            if used_volume + item_volume > *max_volume {
                let item_name = get_reference_name(notification.contents.item, world);
                let message = format!("{item_name} is too big.");
                return VerifyResult::invalid(
                    notification.notification_type.performing_entity,
                    GameMessage::Error(message),
                );
            }
        }
    }

    VerifyResult::valid()
}
