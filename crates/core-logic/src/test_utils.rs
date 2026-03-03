use crate::{
    component::{Container, Description, Location},
    GameMessage, Time,
};
use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};

/// Information about a player created in unit tests
pub struct TestPlayer {
    /// The player's entity in the world
    pub entity: Entity,
    /// The player's command sender
    pub command_sender: Sender<String>,
    /// The player's message receiver
    pub message_receiver: Receiver<(GameMessage, Time)>,
}

pub fn spawn_entity_in_location(id: &str, location: Entity, world: &mut World) -> Entity {
    use crate::move_entity;

    let entity = world.spawn(build_entity_description(id)).id();
    move_entity(entity, location, world);
    entity
}

pub fn build_entity_description(id: &str) -> Description {
    use crate::Pronouns;

    //TODO remove "entity" from the beginning of all these strings
    Description {
        name: format!("entity {id} name"),
        room_name: format!("entity {id} room name"),
        plural_name: format!("entity {id} plural name"),
        article: Some("an".to_string()),
        pronouns: Pronouns::it(),
        aliases: vec![
            format!("entity {id} alias 1"),
            format!("entity {id} alias 2"),
        ],
        description: format!("entity {id} description"),
        attribute_describers: Vec::new(),
    }
}

/// Finds the entity with the provided name. Panics if a matching entity is not found.
/// Can be helpful to avoid capturing variables in closures so they can be coerced into `fn` types.
pub fn get_entity_by_name<'w>(name: &'static str, world: &'w World) -> EntityRef<'w> {
    world
        .iter_entities()
        .find(|e| {
            world
                .get::<Description>(e.id())
                .map(|d| d.name == name)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| panic!("entity with name {name} should exist"))
}

/// Asserts that `entity` is in `containing_entity`.
pub fn assert_entity_in_container(entity: Entity, containing_entity: Entity, world: &World) {
    let location = world.get::<Location>(entity).unwrap();
    assert_eq!(containing_entity, location.id);

    let container = world.get::<Container>(containing_entity).unwrap();
    assert!(container
        .get_entities_including_invisible()
        .contains(&entity));
}

/// Asserts that `entity` is not in `not_containing_entity`.
pub fn assert_entity_not_in_container(
    entity: Entity,
    not_containing_entity: Entity,
    world: &World,
) {
    let location = world.get::<Location>(entity).unwrap();
    assert_ne!(not_containing_entity, location.id);

    let container = world.get::<Container>(not_containing_entity).unwrap();
    assert!(!container
        .get_entities_including_invisible()
        .contains(&entity));
}
