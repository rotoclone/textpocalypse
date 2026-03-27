use bevy_ecs::prelude::*;

use crate::component::StatAdjustments;

mod hungry;
use hungry::*;

/// Registers notification handlers related to status effects.
pub fn register_status_effect_handlers(world: &mut World) {
    Hungry::register_notification_handlers(world);
}

#[derive(Debug, Clone)]
pub struct StatusEffectDetails {
    /// The name of the status effect
    pub name: String,
    /// Any stat adjustments applied by the status effect
    pub stat_adjustments: StatAdjustments,
    /// A description of any other effects the status effect has
    pub other_effects: Vec<String>,
}

trait StatusEffect {
    /// Registers any notification handlers for this status effect.
    fn register_notification_handlers(world: &mut World);
    /// Gets a description of the status effect.
    fn get_details(&self) -> StatusEffectDetails;
    /// Adds this status effect to an entity.
    fn add_to(self, entity: Entity, world: &mut World);
    /// Removes this status effect from an entity.
    fn remove_from(entity: Entity, world: &mut World);
}
