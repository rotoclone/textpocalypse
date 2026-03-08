use bevy_ecs::prelude::*;

use crate::{
    notification::Notification, vital_change::VitalChangedNotification, ConstrainedValue, VitalType,
};

/// The fraction of total satiety at which an entity first becomes hungry
const MILD_HUNGER_THRESHOLD: f32 = 0.5;

/// The fraction of total satiety at which an entity becomes severely hungry
const SEVERE_HUNGER_THESHOLD: f32 = 0.25;

#[derive(Component)]
struct Hungry(HungerSeverity);

#[derive(PartialEq, Eq)]
enum HungerSeverity {
    /// A little hungry
    Mild,
    /// Very hungry
    Severe,
}

/// Adds, removes, or modifies the `Hungry` component based on how hungry an entity is.
pub fn add_or_remove_hungry(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;

    if let VitalType::Satiety = notification.notification_type.vital_type {
        let mut entity_mut = world.entity_mut(entity);
        if let Some(severity) = determine_hunger_severity(notification.notification_type.new_value)
        {
            entity_mut.insert(Hungry(severity));
        } else {
            entity_mut.remove::<Hungry>();
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

//TODO add something to modify stats if an entity is hungry
