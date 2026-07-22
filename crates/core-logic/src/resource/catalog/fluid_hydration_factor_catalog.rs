use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{
    action::DrinkAction,
    component::{AfterActionPerformNotification, FluidType},
    notification::Notification,
    resource::catalog::{Catalog, CatalogBoilerplate},
    vital_change::{
        ValueChangeOperation, VitalChange, VitalChangeMessageParams, VitalChangeVisualizationType,
        VitalType,
    },
    InternalMessageCategory, MessageCategory, NoTokens,
};

/// The amount of hydration gain per liter of fluid drank that the hydration factors are in relation to.
const BASE_HYDRATION_GAIN_PER_LITER: f32 = 50.0;

/// Map of fluids to the amount of hydration drinking that fluid provides, compared to pure water.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(FluidType)]
pub struct FluidHydrationFactorCatalog {
    standard: HashMap<FluidType, f32>,
    custom: HashMap<String, f32>,
}

impl Catalog<FluidType> for FluidHydrationFactorCatalog {
    type V = f32;

    fn get_default_value(thing: &FluidType) -> Option<Self::V> {
        match thing {
            FluidType::Water => Some(1.0),
            FluidType::DirtyWater => Some(0.9),
            FluidType::Alcohol => Some(0.5),
            FluidType::Custom(_) => None,
        }
    }

    fn get_not_found_value() -> Self::V {
        0.0
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

                volume.0 * BASE_HYDRATION_GAIN_PER_LITER * hydration_factor
            })
            .sum::<f32>();

        if hydration_increase > 0.0 {
            VitalChange::<NoTokens> {
                entity: notification.notification_type.performing_entity,
                vital_type: VitalType::Hydration,
                operation: ValueChangeOperation::Add,
                amount: hydration_increase,
                message_params: vec![(
                    VitalChangeMessageParams::Direct {
                        entity: notification.notification_type.performing_entity,
                        message: "Refreshing!".to_string(),
                        category: MessageCategory::Internal(InternalMessageCategory::Misc),
                    },
                    VitalChangeVisualizationType::Full,
                )],
            }
            .apply(world);
        }
    }
}
