use bevy_ecs::prelude::*;

use crate::{
    component::{
        Container, Location, StatAdjustments, StatusEffect, StatusEffectDetails, StatusEffectId,
    },
    is_living_entity,
    notification::{Notification, NotificationHandlers},
    DespawnNotification, EntityMovedNotification,
};

const STATUS_EFFECT_ID: StatusEffectId = StatusEffectId("overencumbered");

/// A status effect applied when an entity's inventory is overfilled.
#[derive(Component)]
pub struct Overencumbered;

impl StatusEffect for Overencumbered {
    fn register_notification_handlers(world: &mut World) {
        NotificationHandlers::add_handler(add_or_remove_overencumbered_on_move, world);
        NotificationHandlers::add_handler(add_or_remove_overencumbered_on_despawn, world);
    }

    fn get_id() -> StatusEffectId {
        STATUS_EFFECT_ID
    }

    fn get_details(&self) -> StatusEffectDetails {
        StatusEffectDetails {
            name: "Overencumbered".to_string(),
            stat_adjustments: StatAdjustments::new(),
            other_effects: vec!["cannot move".to_string()],
        }
    }

    fn on_add(&self, _entity: Entity, _world: &mut World) {
        // nothing extra to do
    }

    fn on_remove(_entity: Entity, _world: &mut World) {
        // nothing extra to do
    }
}

/// Adds or removes `Overencumbered` from entities when an entity moves.
fn add_or_remove_overencumbered_on_move(
    notification: &Notification<EntityMovedNotification, ()>,
    world: &mut World,
) {
    if let Some(source) = notification.notification_type.source {
        add_or_remove_overencumbered_for_entity(source, world);
    }

    add_or_remove_overencumbered_for_entity(notification.notification_type.destination, world);
}

/// Adds or removes `Overencumbered` from containing entities when an entity in them despawns.
fn add_or_remove_overencumbered_on_despawn(
    notification: &Notification<DespawnNotification, ()>,
    world: &mut World,
) {
    if let Some(location) = world.get::<Location>(notification.notification_type.entity) {
        add_or_remove_overencumbered_for_entity(location.id, world);
    }
}

/// Adds or removes `Overencumbered` from an entity based on whether its inventory is overfull or not.
///
/// Does nothing if `entity` isn't a living entity.
fn add_or_remove_overencumbered_for_entity(entity: Entity, world: &mut World) {
    if !is_living_entity(entity, world) {
        return;
    }

    let Some(container) = world.get::<Container>(entity) else {
        return;
    };

    let mut over_weight = false;
    let mut over_volume = false;

    if let Some(max_weight) = container.max_weight {
        if container.used_weight(world) > max_weight {
            over_weight = true;
        }
    }

    if let Some(volume) = container.volume {
        if container.used_volume(world) > volume {
            over_volume = true;
        }
    }

    if over_weight || over_volume {
        Overencumbered.add_to(entity, world);
    } else {
        Overencumbered::remove_from(entity, world);
    }
}
