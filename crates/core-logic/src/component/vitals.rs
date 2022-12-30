use bevy_ecs::prelude::*;

use crate::{
    interrupt_entity, kill_entity,
    notification::Notification,
    send_message,
    value_change::{ValueChange, ValueChangeOperation, ValueChangedNotification, ValueType},
    ConstrainedValue, GameMessage, MessageDelay, TickNotification, ValueChangeDescription,
};

const SATIETY_LOSS_PER_TICK: f32 = 0.005; // loss of 100 satiety in ~3 days
const HYDRATION_LOSS_PER_TICK: f32 = 0.008; // loss of 100 hydration in ~2 days
const ENERGY_LOSS_PER_TICK: f32 = 0.015; // loss of 100 energy in ~1 day

const STARVATION_DAMAGE_PER_TICK: f32 = 5.0;
const THIRST_DAMAGE_PER_TICK: f32 = 50.0; // TODO 5.0;

const HUNGER_MESSAGES: [(f32, &str); 4] = [
    (0.75, "You start feeling a little hungry."),
    (0.66, "You feel hungry."),
    (0.5, "You feel very hungry."),
    (0.25, "You feel extremely hungry."),
];
const THIRST_MESSAGES: [(f32, &str); 4] = [
    (0.75, "You start feeling a little thirsty."),
    (0.66, "You feel thirsty."),
    (0.5, "You feel very thirsty."),
    (0.25, "You feel extremely thirsty."),
];
const TIREDNESS_MESSAGES: [(f32, &str); 4] = [
    (0.66, "You start feeling a little tired."),
    (0.5, "You feel tired."),
    (0.33, "You feel very tired."),
    (0.15, "You feel extremely tired."),
];

/// The vital stats of an entity.
///
/// These values should not be mutated directly; use `ValueChange` for that.
#[derive(Debug, Clone, Component)]
pub struct Vitals {
    /// How healthy the entity is.
    pub health: ConstrainedValue<f32>,
    /// How non-hungry the entity is.
    pub satiety: ConstrainedValue<f32>,
    /// How non-thirsty the entity is.
    pub hydration: ConstrainedValue<f32>,
    /// How non-tired the entity is.
    pub energy: ConstrainedValue<f32>,
}

impl Default for Vitals {
    fn default() -> Self {
        Self::new()
    }
}

impl Vitals {
    /// Creates the default set of vitals.
    pub fn new() -> Vitals {
        Vitals {
            health: ConstrainedValue::new_max(0.0, 100.0),
            satiety: ConstrainedValue::new_max(0.0, 100.0),
            hydration: ConstrainedValue::new_max(0.0, 100.0),
            energy: ConstrainedValue::new_max(0.0, 100.0),
        }
    }
}

/// Decreases vitals over time.
pub fn reduce_vitals_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut value_changes = Vec::new();
    let mut query = world.query::<(Entity, &Vitals)>();
    for (entity, vitals) in query.iter(world) {
        if vitals.satiety.get() <= 0.0 {
            value_changes.push(ValueChange {
                entity,
                value_type: ValueType::Health,
                operation: ValueChangeOperation::Subtract,
                amount: STARVATION_DAMAGE_PER_TICK,
                message: Some("You're starving to death!".to_string()),
            });
        }

        if vitals.hydration.get() <= 0.0 {
            value_changes.push(ValueChange {
                entity,
                value_type: ValueType::Health,
                operation: ValueChangeOperation::Subtract,
                amount: THIRST_DAMAGE_PER_TICK,
                message: Some("You're dying of thirst!".to_string()),
            });
        }

        value_changes.push(ValueChange {
            entity,
            value_type: ValueType::Satiety,
            operation: ValueChangeOperation::Subtract,
            amount: SATIETY_LOSS_PER_TICK,
            message: None,
        });

        value_changes.push(ValueChange {
            entity,
            value_type: ValueType::Hydration,
            operation: ValueChangeOperation::Subtract,
            amount: HYDRATION_LOSS_PER_TICK,
            message: None,
        });

        value_changes.push(ValueChange {
            entity,
            value_type: ValueType::Energy,
            operation: ValueChangeOperation::Subtract,
            amount: ENERGY_LOSS_PER_TICK,
            message: None,
        });
    }

    value_changes
        .into_iter()
        .for_each(|change| change.apply(world));
}

/// Sends update messages when vitals reach certain thresholds.
pub fn send_vitals_update_messages(
    notification: &Notification<ValueChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let value_type = notification.notification_type.value_type;
    let old_value = &notification.notification_type.old_value;
    let new_value = &notification.notification_type.new_value;

    let messages: &[(f32, &str)] = match value_type {
        ValueType::Health => &[],
        ValueType::Satiety => &HUNGER_MESSAGES,
        ValueType::Hydration => &THIRST_MESSAGES,
        ValueType::Energy => &TIREDNESS_MESSAGES,
    };

    let old_fraction = old_value.get() / old_value.get_max();
    let new_fraction = new_value.get() / new_value.get_max();

    for (fraction, message) in messages.iter().rev() {
        if old_fraction > *fraction && new_fraction <= *fraction {
            send_message(
                world,
                entity,
                GameMessage::ValueChange(
                    ValueChangeDescription {
                        message: message.to_string(),
                        value_type,
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    },
                    MessageDelay::Short,
                ),
            );
            break;
        }
    }
}

/// Sets entities' actions to be interrupted when they take damage.
pub fn interrupt_on_damage(
    notification: &Notification<ValueChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let value_type = notification.notification_type.value_type;
    let old_value = &notification.notification_type.old_value;
    let new_value = &notification.notification_type.new_value;

    if let ValueType::Health = value_type {
        if new_value.get() < old_value.get() {
            interrupt_entity(entity, world);
        }
    }
}

/// Kills entities when they reach 0 health.
pub fn kill_on_zero_health(
    notification: &Notification<ValueChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let value_type = notification.notification_type.value_type;
    let new_value = &notification.notification_type.new_value;

    if let ValueType::Health = value_type {
        if new_value.get() <= 0.0 {
            kill_entity(entity, world);
        }
    }
}
