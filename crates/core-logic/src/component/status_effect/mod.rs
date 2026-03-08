use bevy_ecs::prelude::*;

use crate::notification::NotificationHandlers;

mod hungry;
use hungry::*;

/// Registers notification handlers related to status effects.
pub fn register_status_effect_handlers(world: &mut World) {
    NotificationHandlers::add_handler(add_or_remove_hungry, world);
}
