use bevy_ecs::prelude::*;

use super::{Container, WornItems};

/// The location of an entity.
#[derive(Component)]
pub enum Location {
    /// The entity is in a container.
    Container(Entity),
    /// The entity is being worn by something.
    Worn(Entity),
}

/// The actual specific data structure containing an entity.
pub enum ConcreteLocation<'a> {
    /// A container on an entity.
    Container(Entity, &'a Container),
    /// A set of worn items on an entity.
    Worn(Entity, &'a WornItems),
}

/// Gets the concrete location of the provided entity.
///
/// Returns `None` if the provided entity doesn't have a location.
///
/// Panics if the entity referenced in the provided entity's location does not have a matching component for the location type.
pub fn get_concrete_location<'w>(entity: Entity, world: &'w World) -> Option<ConcreteLocation<'w>> {
    if let Some(location) = world.get::<Location>(entity) {
        match location {
            Location::Container(e) => {
                return Some(ConcreteLocation::Container(
                    *e,
                    world
                        .get::<Container>(*e)
                        .expect("location should be a container"),
                ))
            }
            Location::Worn(e) => {
                return Some(ConcreteLocation::Worn(
                    *e,
                    world
                        .get::<WornItems>(*e)
                        .expect("location should have worn items"),
                ))
            }
        }
    }

    None
}

/// Mutably gets the concrete location of the provided entity.
///
/// Returns `None` if the provided entity doesn't have a location.
///
/// Panics if the entity referenced in the provided entity's location does not have a matching component for the location type.
pub fn get_concrete_location_mut<'w>(
    entity: Entity,
    world: &'w mut World,
) -> Option<ConcreteLocation<'w>> {
    if let Some(location) = world.get::<Location>(entity) {
        match location {
            Location::Container(e) => {
                return Some(ConcreteLocation::Container(
                    *e,
                    &world
                        .get_mut::<Container>(*e)
                        .expect("location should be a container"),
                ))
            }
            Location::Worn(e) => {
                return Some(ConcreteLocation::Worn(
                    *e,
                    &world
                        .get_mut::<WornItems>(*e)
                        .expect("location should have worn items"),
                ))
            }
        }
    }

    None
}

/// Gets the container the provided entity is in.
///
/// Returns `None` if the entity doesn't have a location, or is not in a container.
pub fn get_containing_container(entity: Entity, world: &World) -> Option<&Container> {
    todo!() //TODO
}

/// Gets the ID of the container the provided entity is in.
///
/// Returns `None` if the entity doesn't have a location, or is not in a container.
pub fn get_container_id(entity: Entity, world: &World) -> Option<Entity> {
    if let Some(Location::Container(id)) = world.get::<Location>(entity) {
        Some(*id)
    } else {
        None
    }
}
