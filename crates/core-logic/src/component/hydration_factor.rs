use bevy_ecs::prelude::*;

use crate::{
    action::DrinkAction,
    notification::Notification,
    value_change::{ValueChange, ValueChangeOperation},
    ValueType,
};

use super::AfterActionNotification;

/// The amount of hydration gain per liter of water drank.
const HYDRATION_GAIN_PER_LITER_OF_WATER: f32 = 50.0;

/// The amount of hydration drinking an entity provides, compared to pure water.
#[derive(Component)]
pub struct HydrationFactor(pub f32);

/// Increases hydration when an entity is drank based on its hydration factor.
pub fn increase_hydration_on_drink(
    notification: &Notification<AfterActionNotification, DrinkAction>,
    world: &mut World,
) {
    if notification.notification_type.action_complete
        && notification.notification_type.action_successful
    {
        let hydration_increase = notification
            .contents
            .fluids_to_volume_drank
            .iter()
            .map(|(entity, volume)| {
                let hydration_factor = world
                    .get::<HydrationFactor>(*entity)
                    .unwrap_or(&HydrationFactor(0.0));

                volume.0 * HYDRATION_GAIN_PER_LITER_OF_WATER * hydration_factor.0
            })
            .sum::<f32>();

        if hydration_increase > 0.0 {
            ValueChange {
                entity: notification.notification_type.performing_entity,
                value_type: ValueType::Hydration,
                operation: ValueChangeOperation::Add,
                amount: hydration_increase,
                message: Some("Refreshing!".to_string()),
            }
            .apply(world);
        }
    }
}
