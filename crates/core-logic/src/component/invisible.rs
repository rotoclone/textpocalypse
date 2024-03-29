use std::collections::HashSet;

use bevy_ecs::prelude::*;

/// Marks an entity as invisible to certain other entities.
#[derive(Component)]
pub struct Invisible {
    /// The scope describing which entities this one is hidden from.
    scope: InvisibilityScope,
}

/// Describes which entities an invisible entity is hidden from.
pub enum InvisibilityScope {
    /// Invisible to all entities.
    All,
    /// Invisible to only certain entities.
    Entities(HashSet<Entity>),
    /// Visible to only certain entities.
    AllExcept(HashSet<Entity>),
}

impl Invisible {
    /// Constructs an `Invisible` that makes an entity hidden from all other entities.
    pub fn to_all() -> Invisible {
        Invisible {
            scope: InvisibilityScope::All,
        }
    }

    /// Constructs an `Invisible` that makes an entity hidden from the provided entities.
    pub fn to_entities(entities: HashSet<Entity>) -> Invisible {
        Invisible {
            scope: InvisibilityScope::Entities(entities),
        }
    }

    /// Constructs an `Invisible` that makes an entity hidden from the provided entity.
    pub fn to_entity(entity: Entity) -> Invisible {
        Invisible::to_entities([entity].into())
    }

    /// Constructs an `Invisible` that makes an entity hidden from all entities except for the provided entities.
    pub fn to_all_except(entities: HashSet<Entity>) -> Invisible {
        Invisible {
            scope: InvisibilityScope::AllExcept(entities),
        }
    }

    /// Makes an entity invisible to all other entities.
    pub fn make_invisible_to_all(entity: Entity, world: &mut World) {
        world.entity_mut(entity).insert(Invisible::to_all());
    }

    /// Makes an entity visible to all other entities.
    pub fn make_visible_to_all(entity: Entity, world: &mut World) {
        world.entity_mut(entity).remove::<Invisible>();
    }

    /// Makes `entity` invisible to `pov_entity`.
    pub fn make_invisible_to(entity: Entity, pov_entity: Entity, world: &mut World) {
        if let Some(mut invisible) = world.get_mut::<Invisible>(entity) {
            match invisible.scope {
                InvisibilityScope::All => (),
                InvisibilityScope::Entities(ref mut entities) => {
                    entities.insert(pov_entity);
                }
                InvisibilityScope::AllExcept(ref mut entities) => {
                    entities.remove(&pov_entity);
                }
            }
        } else {
            world
                .entity_mut(entity)
                .insert(Invisible::to_entity(pov_entity));
        }
    }

    /// Makes `entity` visible to `pov_entity`.
    pub fn make_visible_to(entity: Entity, pov_entity: Entity, world: &mut World) {
        if let Some(mut invisible) = world.get_mut::<Invisible>(entity) {
            match invisible.scope {
                InvisibilityScope::All => {
                    *invisible = Invisible::to_all_except([pov_entity].into())
                }
                InvisibilityScope::Entities(ref mut entities) => {
                    entities.remove(&pov_entity);
                }
                InvisibilityScope::AllExcept(ref mut entities) => {
                    entities.insert(pov_entity);
                }
            }
        }
    }

    /// Returns `true` if `entity` is invisible to `pov_entity`, `false` otherwise.
    pub fn is_invisible_to(entity: Entity, pov_entity: Entity, world: &World) -> bool {
        if let Some(invisible) = world.get::<Invisible>(entity) {
            match &invisible.scope {
                InvisibilityScope::All => true,
                InvisibilityScope::Entities(entities) => entities.contains(&pov_entity),
                InvisibilityScope::AllExcept(entities) => !entities.contains(&pov_entity),
            }
        } else {
            false
        }
    }

    /// Returns `true` if `entity` is visible to `pov_entity`, `false` otherwise.
    pub fn is_visible_to(entity: Entity, pov_entity: Entity, world: &World) -> bool {
        !Invisible::is_invisible_to(entity, pov_entity, world)
    }
}
