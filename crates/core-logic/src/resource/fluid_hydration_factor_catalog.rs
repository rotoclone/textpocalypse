use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    action::DrinkAction,
    component::{AfterActionPerformNotification, FluidType},
    notification::Notification,
    swap_tuple::swapped,
    vital_change::{
        ValueChangeOperation, VitalChange, VitalChangeMessageParams, VitalChangeVisualizationType,
        VitalType,
    },
};

/// The amount of hydration gain per liter of water drank.
const HYDRATION_GAIN_PER_LITER_OF_WATER: f32 = 50.0;

/// Map of fluids to the amount of hydration drinking that fluid provides, compared to pure water.
#[derive(Resource)]
pub struct FluidHydrationFactorCatalog {
    standard: HashMap<FluidType, f32>,
    custom: HashMap<String, f32>,
}

impl FluidHydrationFactorCatalog {
    /// Creates the default catalog of hydration factors.
    pub fn new() -> FluidHydrationFactorCatalog {
        FluidHydrationFactorCatalog {
            standard: build_standard_hydration_factors(),
            custom: HashMap::new(),
        }
    }

    /// Sets the hydration factor of the provided fluid type.
    pub fn set(&mut self, fluid_type: &FluidType, factor: f32) {
        match fluid_type {
            FluidType::Custom(id) => self.custom.insert(id.clone(), factor),
            _ => self.standard.insert(fluid_type.clone(), factor),
        };
    }

    /// Determines the hydration factor for the provided fluid type.
    pub fn get(&self, fluid_type: &FluidType) -> f32 {
        match fluid_type {
            FluidType::Custom(id) => *self.custom.get(id).unwrap_or(&0.0),
            _ => *self.standard.get(fluid_type).unwrap_or(&0.0),
        }
    }
}

/// Builds the default hydration factors of standard fluid types.
fn build_standard_hydration_factors() -> HashMap<FluidType, f32> {
    FluidType::iter()
        .map(|fluid_type| swapped(get_default_hydration_factor(&fluid_type), fluid_type))
        .collect()
}

/// Gets the default hydration factor of a fluid type.
fn get_default_hydration_factor(fluid_type: &FluidType) -> f32 {
    match fluid_type {
        FluidType::Water => 1.0,
        FluidType::DirtyWater => 0.9,
        FluidType::Alcohol => 0.5,
        FluidType::Custom(_) => 0.0,
    }
}

/// Increases hydration when an entity is drank based on its hydration factor.
pub fn increase_hydration_on_drink(
    notification: &Notification<AfterActionPerformNotification, DrinkAction>,
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
                    .resource::<FluidHydrationFactorCatalog>()
                    .get(fluid_type);

                volume.0 * HYDRATION_GAIN_PER_LITER_OF_WATER * hydration_factor
            })
            .sum::<f32>();

        if hydration_increase > 0.0 {
            VitalChange {
                entity: notification.notification_type.performing_entity,
                vital_type: VitalType::Hydration,
                operation: ValueChangeOperation::Add,
                amount: hydration_increase,
                message_params: vec![VitalChangeMessageParams {
                    entity: notification.notification_type.performing_entity,
                    message: "Refreshing!".to_string(),
                    visualization_type: VitalChangeVisualizationType::Full,
                }],
            }
            .apply(world);
        }
    }
}
