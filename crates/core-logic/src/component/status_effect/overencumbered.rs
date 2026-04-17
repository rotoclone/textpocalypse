use bevy_ecs::prelude::*;

use crate::component::{StatAdjustments, StatusEffect, StatusEffectDetails, StatusEffectId};

const STATUS_EFFECT_ID: StatusEffectId = StatusEffectId("overencumbered");

/// A status effect applied when an entity's inventory is overfilled.
#[derive(Component)]
pub struct Overencumbered;

impl StatusEffect for Overencumbered {
    fn register_notification_handlers(world: &mut World) {
        todo!() //TODO
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

    fn on_add(&self, entity: Entity, world: &mut World) {
        // nothing extra to do
    }

    fn on_remove(entity: Entity, world: &mut World) {
        // nothing extra to do
    }
}
