use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::get_or_insert_mut;

/// Describes who an entity is in combat with.
#[derive(Component, Default)]
pub struct CombatState {
    /// The entities this entity is currently in combat with.
    //TODO this should also include `CombatRange`s
    pub entities_in_combat_with: HashSet<Entity>,
}

impl CombatState {
    /// Finds all the entities the provided entity is currently in combat with.
    /// If the entity is not in combat, an empty set will be returned.
    pub fn get_entities_in_combat_with(entity: Entity, world: &World) -> HashSet<Entity> {
        world
            .get::<CombatState>(entity)
            .map(|combat_state| combat_state.entities_in_combat_with.clone())
            .unwrap_or_default()
    }

    /// Marks the provided entities as in combat with each other.
    pub fn enter_combat(entity_1: Entity, entity_2: Entity, world: &mut World) {
        let mut entity_1_combat_state = get_or_insert_mut::<CombatState>(entity_1, world);
        entity_1_combat_state
            .entities_in_combat_with
            .insert(entity_2);

        let mut entity_2_combat_state = get_or_insert_mut::<CombatState>(entity_2, world);
        entity_2_combat_state
            .entities_in_combat_with
            .insert(entity_1);
    }

    /// Marks the provided entities as not in combat with each other.
    pub fn leave_combat(entity_1: Entity, entity_2: Entity, world: &mut World) {
        let mut entity_1_combat_state = get_or_insert_mut::<CombatState>(entity_1, world);
        entity_1_combat_state
            .entities_in_combat_with
            .remove(&entity_2);

        let mut entity_2_combat_state = get_or_insert_mut::<CombatState>(entity_2, world);
        entity_2_combat_state
            .entities_in_combat_with
            .remove(&entity_1);
    }

    /// Marks the provided entity as not in combat with anyone.
    pub fn leave_all_combat(entity: Entity, world: &mut World) {
        for other_entity in CombatState::get_entities_in_combat_with(entity, world) {
            Self::leave_combat(entity, other_entity, world);
        }
    }
}

//TODO remove entities from combat when they die
