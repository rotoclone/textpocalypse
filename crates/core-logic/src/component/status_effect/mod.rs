use bevy_ecs::prelude::*;

use crate::{
    component::StatAdjustments,
    notification::{
        ReturningNotificationHandleFn, ReturningNotificationHandlers, ReturningNotificationType,
    },
};

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
    fn register_notification_handlers(world: &mut World) {
        Self::register_add_and_remove_handlers(world);
        ReturningNotificationHandlers::add_handler(Self::get_description_handler(), world);
    }

    /// Registers notification handler(s) for adding and removing the status effect.
    fn register_add_and_remove_handlers(world: &mut World);
    /// Gets the notification handler function that gets this status effect's description.
    fn get_description_handler(
    ) -> ReturningNotificationHandleFn<GetStatusEffects, (), Option<StatusEffectDetails>>;
    /// Gets a description of the status effect.
    fn get_details(&self) -> StatusEffectDetails;
    /// Adds this status effect to an entity.
    fn add_to(self, entity: Entity, world: &mut World);
    /// Removes this status effect from an entity.
    fn remove_from(entity: Entity, world: &mut World);
}

/// Notification type for getting the active status effects of an entity.
/// TODO should this just be a StatusEffects component that keeps track of all of them instead? that way getting status effect info is more efficient
#[derive(Debug)]
pub struct GetStatusEffects {
    /// The entity to get status effects for.
    pub entity: Entity,
}

impl ReturningNotificationType for GetStatusEffects {
    type Return = Option<StatusEffectDetails>;
}
