use std::{collections::HashMap, fmt::Display};

use bevy_ecs::prelude::*;

use crate::{
    get_or_insert_mut, notification::Notification, DeathNotification, DespawnNotification,
    NotificationType,
};

/// Describes who an entity is in combat with.
#[derive(Component, Default)]
pub struct CombatState {
    /// The entities this entity is currently in combat with, and the ranges to them.
    entities_in_combat_with: HashMap<Entity, CombatRange>,
}

/// Represents how far away two combatants are from each other.
#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum CombatRange {
    Shortest,
    Short,
    Medium,
    Long,
    Longest,
}

/// A notification that two entities have started fighting.
#[derive(Debug)]
pub struct EnterCombatNotification {
    /// One of the entities entering combat.
    pub entity_1: Entity,
    /// The other entity entering combat.
    pub entity_2: Entity,
}

impl NotificationType for EnterCombatNotification {}

/// A notification that two entities have stopped fighting.
#[derive(Debug)]
pub struct ExitCombatNotification {
    /// One of the entities exiting combat.
    pub entity_1: Entity,
    /// The other entity exiting combat.
    pub entity_2: Entity,
}

impl NotificationType for ExitCombatNotification {}

impl CombatState {
    /// Finds all the entities the provided entity is currently in combat with.
    /// If the entity is not in combat, an empty map will be returned.
    pub fn get_entities_in_combat_with(
        entity: Entity,
        world: &World,
    ) -> HashMap<Entity, CombatRange> {
        world
            .get::<CombatState>(entity)
            .map(|combat_state| combat_state.entities_in_combat_with.clone())
            .unwrap_or_default()
    }

    /// Marks the provided entities as in combat with each other at the provided range.
    pub fn set_in_combat(
        entity_1: Entity,
        entity_2: Entity,
        range: CombatRange,
        world: &mut World,
    ) {
        let mut entity_1_combat_state = get_or_insert_mut::<CombatState>(entity_1, world);
        entity_1_combat_state
            .entities_in_combat_with
            .insert(entity_2, range);

        let mut entity_2_combat_state = get_or_insert_mut::<CombatState>(entity_2, world);
        entity_2_combat_state
            .entities_in_combat_with
            .insert(entity_1, range);

        Notification::send_no_contents(EnterCombatNotification { entity_1, entity_2 }, world);
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

        Notification::send_no_contents(ExitCombatNotification { entity_1, entity_2 }, world);
    }

    /// Marks the provided entity as not in combat with anyone.
    pub fn leave_all_combat(entity: Entity, world: &mut World) {
        for (other_entity, _) in CombatState::get_entities_in_combat_with(entity, world) {
            Self::leave_combat(entity, other_entity, world);
        }
    }

    /// Gets all the entities and ranges in this combat state.
    pub fn get_entities(&self) -> &HashMap<Entity, CombatRange> {
        &self.entities_in_combat_with
    }
}

impl Display for CombatRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CombatRange::Shortest => "shortest".fmt(f),
            CombatRange::Short => "short".fmt(f),
            CombatRange::Medium => "medium".fmt(f),
            CombatRange::Long => "long".fmt(f),
            CombatRange::Longest => "longest".fmt(f),
        }
    }
}

impl CombatRange {
    /// Returns the range one level farther than this one, or `None` if this is the farthest range.
    pub fn increased(&self) -> Option<CombatRange> {
        match self {
            CombatRange::Shortest => Some(CombatRange::Short),
            CombatRange::Short => Some(CombatRange::Medium),
            CombatRange::Medium => Some(CombatRange::Long),
            CombatRange::Long => Some(CombatRange::Longest),
            CombatRange::Longest => None,
        }
    }

    /// Returns the range one level closer than this one, or `None` if this is the closest range.
    pub fn decreased(&self) -> Option<CombatRange> {
        match self {
            CombatRange::Shortest => None,
            CombatRange::Short => Some(CombatRange::Shortest),
            CombatRange::Medium => Some(CombatRange::Short),
            CombatRange::Long => Some(CombatRange::Medium),
            CombatRange::Longest => Some(CombatRange::Long),
        }
    }
}

// Removes entities from combat when they die.
pub fn remove_from_combat_on_death(
    notification: &Notification<DeathNotification, ()>,
    world: &mut World,
) {
    CombatState::leave_all_combat(notification.notification_type.entity, world);
}

// Removes entities from combat when they despawn.
pub fn remove_from_combat_on_despawn(
    notification: &Notification<DespawnNotification, ()>,
    world: &mut World,
) {
    CombatState::leave_all_combat(notification.notification_type.entity, world);
}
