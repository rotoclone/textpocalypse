use crate::component::Description;
use bevy_ecs::prelude::*;

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
