use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::{
    action::DrinkAction,
    component::{AfterActionNotification, FluidType},
    notification::Notification,
    value_change::{ValueChange, ValueChangeOperation},
    ValueType,
};

/// The amount of hydration gain per liter of water drank.
const HYDRATION_GAIN_PER_LITER_OF_WATER: f32 = 50.0;

/// Map of fluids to the amount of hydration drinking that fluid provides, compared to pure water.
#[derive(Resource)]
pub struct FluidHydrationFactors {
    standard: HashMap<FluidType, f32>,
    custom: HashMap<String, f32>,
}

impl FluidHydrationFactors {
    /// Creates the default store of hydration factors.
    pub fn new() -> FluidHydrationFactors {
        FluidHydrationFactors {
            standard: [
                (FluidType::Water, 1.0),
                (FluidType::DirtyWater, 0.9),
                (FluidType::Alcohol, 0.5),
            ]
            .into(),
            custom: HashMap::new(),
        }
    }

    /// Determines the hydration factor for the provided fluid type.
    pub fn for_fluid(&self, fluid_type: &FluidType) -> f32 {
        match fluid_type {
            FluidType::Custom(id) => *self.custom.get(id).unwrap_or(&0.0),
            _ => *self.standard.get(fluid_type).unwrap_or(&0.0),
        }
    }
}

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
            .map(|(fluid_type, volume)| {
                let hydration_factor = world
                    .resource::<FluidHydrationFactors>()
                    .for_fluid(fluid_type);

                volume.0 * HYDRATION_GAIN_PER_LITER_OF_WATER * hydration_factor
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
