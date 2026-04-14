use bevy_ecs::prelude::*;

use crate::{
    component::{
        status_effect::StatusEffect, Attribute, Stat, StatAdjustment, StatAdjustmentKey,
        StatAdjustments, Stats, StatusEffectDetails, StatusEffectId,
    },
    notification::{Notification, NotificationHandlers},
    vital_change::VitalChangedNotification,
    ConstrainedValue, VitalType,
};

/// The fraction of total hydration at which an entity first becomes thirsty
pub const MILD_THIRST_THRESHOLD: f32 = 0.5;

/// The fraction of total hydration at which an entity becomes severely thirsty
pub const SEVERE_THIRST_THESHOLD: f32 = 0.25;

const STATUS_EFFECT_ID: StatusEffectId = StatusEffectId("thirsty");
const STAT_ADJUSTMENT_KEY: StatAdjustmentKey = StatAdjustmentKey("thirsty");

/// A status effect applied when an entity's hydration is low.
#[derive(Component)]
pub struct Thirsty(ThirstSeverity);

/// How severse the thirsty status effect is.
#[derive(PartialEq, Eq)]
enum ThirstSeverity {
    /// A little thirsty
    Mild,
    /// Very thirsty
    Severe,
}

impl Thirsty {
    /// Determines what stat adjustments to apply for this level of thirst.
    fn get_stat_adjustments(&self) -> StatAdjustments {
        match self.0 {
            ThirstSeverity::Mild => StatAdjustments::new().adjust_stat(
                Stat::Attribute(Attribute::Strength),
                StatAdjustment::Subtract(1.0),
            ),
            ThirstSeverity::Severe => StatAdjustments::new().adjust_stat(
                Stat::Attribute(Attribute::Strength),
                StatAdjustment::Subtract(2.0),
            ),
        }
    }
}

impl StatusEffect for Thirsty {
    fn register_notification_handlers(world: &mut World) {
        NotificationHandlers::add_handler(add_or_remove_thirsty, world);
    }

    fn get_id() -> StatusEffectId {
        STATUS_EFFECT_ID
    }

    fn get_details(&self) -> StatusEffectDetails {
        let name = match self.0 {
            ThirstSeverity::Mild => "Thirsty".to_string(),
            ThirstSeverity::Severe => "Very thirsty".to_string(),
        };
        StatusEffectDetails {
            name,
            stat_adjustments: self.get_stat_adjustments(),
            other_effects: Vec::new(),
        }
    }

    fn on_add(&self, entity: Entity, world: &mut World) {
        if let Some(mut stats) = world.get_mut::<Stats>(entity) {
            stats.set_adjustment(STAT_ADJUSTMENT_KEY, self.get_stat_adjustments());
        }
    }

    fn on_remove(entity: Entity, world: &mut World) {
        if let Some(mut stats) = world.get_mut::<Stats>(entity) {
            stats.remove_adjustment(STAT_ADJUSTMENT_KEY);
        }
    }
}

/// Adds, removes, or modifies the `Thirsty` component based on how thirsty an entity is.
fn add_or_remove_thirsty(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;

    if let VitalType::Hydration = notification.notification_type.vital_type {
        if let Some(severity) = determine_thirst_severity(notification.notification_type.new_value)
        {
            Thirsty(severity).add_to(entity, world);
        } else {
            Thirsty::remove_from(entity, world);
        }
    }
}

/// Determines what thirst severity corresponds to the given hydration value.
/// Returns `None` if the hydration doesn't represent a level that's officially "thirsty".
fn determine_thirst_severity(hydration: ConstrainedValue<f32>) -> Option<ThirstSeverity> {
    let fraction = hydration.get() / hydration.get_max();
    match fraction {
        x if x <= SEVERE_THIRST_THESHOLD => Some(ThirstSeverity::Severe),
        x if x <= MILD_THIRST_THRESHOLD => Some(ThirstSeverity::Mild),
        _ => None,
    }
}
