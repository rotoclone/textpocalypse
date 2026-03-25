use bevy_ecs::prelude::*;

use crate::{
    component::{
        status_effect::{StatusEffect, StatusEffectDescription},
        Attribute, Stat, StatAdjustment, StatAdjustmentKey, StatAdjustments, Stats,
    },
    notification::{Notification, NotificationHandlers},
    vital_change::VitalChangedNotification,
    ConstrainedValue, VitalType,
};

/// The fraction of total satiety at which an entity first becomes hungry
const MILD_HUNGER_THRESHOLD: f32 = 0.5;

/// The fraction of total satiety at which an entity becomes severely hungry
const SEVERE_HUNGER_THESHOLD: f32 = 0.25;

const STAT_ADJUSTMENT_KEY: StatAdjustmentKey = StatAdjustmentKey("hungry");

#[derive(Component)]
pub struct Hungry(HungerSeverity);

#[derive(PartialEq, Eq)]
enum HungerSeverity {
    /// A little hungry
    Mild,
    /// Very hungry
    Severe,
}

impl Hungry {
    /// Determines what stat adjustments too apply for this level of hunger.
    fn get_stat_adjustments(&self) -> StatAdjustments {
        match self.0 {
            HungerSeverity::Mild => StatAdjustments::new().adjust_stat(
                Stat::Attribute(Attribute::Strength),
                StatAdjustment::Subtract(1.0),
            ),
            HungerSeverity::Severe => StatAdjustments::new().adjust_stat(
                Stat::Attribute(Attribute::Strength),
                StatAdjustment::Subtract(2.0),
            ),
        }
    }
}

impl StatusEffect for Hungry {
    fn register_notification_handlers(world: &mut World) {
        NotificationHandlers::add_handler(add_or_remove_hungry, world);
    }

    fn get_description(&self) -> StatusEffectDescription {
        let name = match self.0 {
            HungerSeverity::Mild => "Hungry".to_string(),
            HungerSeverity::Severe => "Very hungry".to_string(),
        };
        StatusEffectDescription {
            name,
            stat_adjustments: self.get_stat_adjustments(),
            other_effects: None,
        }
    }

    fn add_to(self, entity: Entity, world: &mut World) {
        if let Some(mut stats) = world.get_mut::<Stats>(entity) {
            stats.set_adjustment(STAT_ADJUSTMENT_KEY, self.get_stat_adjustments());
        }

        world.entity_mut(entity).insert(self);
    }

    fn remove_from(entity: Entity, world: &mut World) {
        if let Some(mut stats) = world.get_mut::<Stats>(entity) {
            stats.remove_adjustment(STAT_ADJUSTMENT_KEY);
        }

        world.entity_mut(entity).remove::<Hungry>();
    }
}

/// Adds, removes, or modifies the `Hungry` component based on how hungry an entity is.
pub fn add_or_remove_hungry(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;

    if let VitalType::Satiety = notification.notification_type.vital_type {
        if let Some(severity) = determine_hunger_severity(notification.notification_type.new_value)
        {
            Hungry(severity).add_to(entity, world);
        } else {
            Hungry::remove_from(entity, world);
        }
    }
}

/// Determines what hunger severity corresponds to the given satiety value.
/// Returns `None` if the satiety doesn't represent a level that's officially "hungry".
fn determine_hunger_severity(satiety: ConstrainedValue<f32>) -> Option<HungerSeverity> {
    let fraction = satiety.get() / satiety.get_max();
    match fraction {
        x if x <= MILD_HUNGER_THRESHOLD => Some(HungerSeverity::Mild),
        x if x <= SEVERE_HUNGER_THESHOLD => Some(HungerSeverity::Severe),
        _ => None,
    }
}
