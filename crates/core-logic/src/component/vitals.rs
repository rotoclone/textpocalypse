use bevy_ecs::prelude::*;

use crate::{
    action::{ActionNotificationSender, SleepAction},
    interrupt_entity, kill_entity,
    notification::Notification,
    send_message,
    vital_change::{ValueChangeOperation, VitalChange, VitalChangedNotification, VitalType},
    ConstrainedValue, GameMessage, MessageDelay, TickNotification, VitalChangeDescription,
};

use super::{is_asleep, queue_action_first};

const SATIETY_LOSS_PER_TICK: f32 = 0.005; // loss of 100 satiety in ~3 days
const HYDRATION_LOSS_PER_TICK: f32 = 0.008; // loss of 100 hydration in ~2 days
const ENERGY_LOSS_PER_TICK: f32 = 0.015; // loss of 100 energy in ~1 day
const ENERGY_GAIN_PER_TICK: f32 = 0.03; // gain of 100 energy in ~12 hours

const STARVATION_DAMAGE_PER_TICK: f32 = 5.0;
const THIRST_DAMAGE_PER_TICK: f32 = 5.0;

const HUNGER_MESSAGES: [ValueChangeMessage; 4] = [
    ValueChangeMessage::decrease(0.75, "You start feeling a little hungry."),
    ValueChangeMessage::decrease(0.66, "You feel hungry."),
    ValueChangeMessage::decrease(0.5, "You feel very hungry."),
    ValueChangeMessage::decrease(0.25, "You feel extremely hungry."),
];
const THIRST_MESSAGES: [ValueChangeMessage; 4] = [
    ValueChangeMessage::decrease(0.75, "You start feeling a little thirsty."),
    ValueChangeMessage::decrease(0.66, "You feel thirsty."),
    ValueChangeMessage::decrease(0.5, "You feel very thirsty."),
    ValueChangeMessage::decrease(0.25, "You feel extremely thirsty."),
];
const TIREDNESS_MESSAGES: [ValueChangeMessage; 4] = [
    ValueChangeMessage::decrease(0.66, "You start feeling a little tired."),
    ValueChangeMessage::decrease(0.5, "You feel tired."),
    ValueChangeMessage::decrease(0.33, "You feel very tired."),
    ValueChangeMessage::decrease(0.15, "You feel extremely tired."),
];
const REST_MESSAGES: [ValueChangeMessage; 5] = [
    ValueChangeMessage::increase(0.15, "You feel a bit less tired."),
    ValueChangeMessage::increase(0.33, "You feel less tired."),
    ValueChangeMessage::increase(0.5, "You only feel a little tired now."),
    ValueChangeMessage::increase(0.66, "You stop feeling tired."),
    ValueChangeMessage::increase(0.9, "You feel very well-rested!"),
];

/// A message to send when a value crosses a certain threshold.
struct ValueChangeMessage {
    /// The threshold, as a fraction of the maximum for the value.
    threshold_fraction: f32,
    /// The direction the value should be going when crossing the threshold.
    direction: ValueChangeDirection,
    /// The message to send if the threshold is crossed.
    message: &'static str,
}

impl ValueChangeMessage {
    /// Creates a message for if a value increases past the threshold fraction.
    pub const fn increase(threshold_fraction: f32, message: &'static str) -> ValueChangeMessage {
        ValueChangeMessage {
            threshold_fraction,
            direction: ValueChangeDirection::Increase,
            message,
        }
    }

    /// Creates a message for if a value decreases past the threshold fraction.
    pub const fn decrease(threshold_fraction: f32, message: &'static str) -> ValueChangeMessage {
        ValueChangeMessage {
            threshold_fraction,
            direction: ValueChangeDirection::Decrease,
            message,
        }
    }
}

#[derive(PartialEq, Eq)]
enum ValueChangeDirection {
    Increase,
    Decrease,
}

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

/// Changes vitals over time.
pub fn change_vitals_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut changes = Vec::new();
    let mut query = world.query::<(Entity, &Vitals)>();
    for (entity, vitals) in query.iter(world) {
        if vitals.satiety.get() <= 0.0 {
            changes.push(VitalChange {
                entity,
                vital_type: VitalType::Health,
                operation: ValueChangeOperation::Subtract,
                amount: STARVATION_DAMAGE_PER_TICK,
                message: Some("You're starving to death!".to_string()),
            });
        }

        if vitals.hydration.get() <= 0.0 {
            changes.push(VitalChange {
                entity,
                vital_type: VitalType::Health,
                operation: ValueChangeOperation::Subtract,
                amount: THIRST_DAMAGE_PER_TICK,
                message: Some("You're dying of thirst!".to_string()),
            });
        }

        //TODO reduce satiety and hydration losses if asleep
        changes.push(VitalChange {
            entity,
            vital_type: VitalType::Satiety,
            operation: ValueChangeOperation::Subtract,
            amount: SATIETY_LOSS_PER_TICK,
            message: None,
        });

        changes.push(VitalChange {
            entity,
            vital_type: VitalType::Hydration,
            operation: ValueChangeOperation::Subtract,
            amount: HYDRATION_LOSS_PER_TICK,
            message: None,
        });

        if is_asleep(entity, world) {
            changes.push(VitalChange {
                entity,
                vital_type: VitalType::Energy,
                operation: ValueChangeOperation::Add,
                amount: ENERGY_GAIN_PER_TICK,
                message: None,
            });
        } else {
            changes.push(VitalChange {
                entity,
                vital_type: VitalType::Energy,
                operation: ValueChangeOperation::Subtract,
                amount: ENERGY_LOSS_PER_TICK,
                message: None,
            });
        }
    }

    changes.into_iter().for_each(|change| change.apply(world));
}

/// Sends update messages when vitals reach certain thresholds.
pub fn send_vitals_update_messages(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let vital_type = notification.notification_type.vital_type;
    let old_value = &notification.notification_type.old_value;
    let new_value = &notification.notification_type.new_value;

    let increased = new_value.get() > old_value.get();
    let messages: &[ValueChangeMessage] = match vital_type {
        VitalType::Health => &[],
        VitalType::Satiety => &HUNGER_MESSAGES,
        VitalType::Hydration => &THIRST_MESSAGES,
        VitalType::Energy => {
            if increased {
                &REST_MESSAGES
            } else {
                &TIREDNESS_MESSAGES
            }
        }
    };

    let old_fraction = old_value.get() / old_value.get_max();
    let new_fraction = new_value.get() / new_value.get_max();

    for message in messages.iter().rev() {
        let should_send = match message.direction {
            ValueChangeDirection::Increase => {
                old_fraction < message.threshold_fraction
                    && new_fraction >= message.threshold_fraction
            }
            ValueChangeDirection::Decrease => {
                old_fraction > message.threshold_fraction
                    && new_fraction <= message.threshold_fraction
            }
        };

        if should_send {
            send_message(
                world,
                entity,
                GameMessage::VitalChange(
                    VitalChangeDescription {
                        message: message.message.to_string(),
                        vital_type,
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
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let vital_type = notification.notification_type.vital_type;
    let old_value = &notification.notification_type.old_value;
    let new_value = &notification.notification_type.new_value;

    if let VitalType::Health = vital_type {
        if new_value.get() < old_value.get() {
            interrupt_entity(entity, world);
        }
    }
}

/// Kills entities when they reach 0 health.
pub fn kill_on_zero_health(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let vital_type = notification.notification_type.vital_type;
    let new_value = &notification.notification_type.new_value;

    if let VitalType::Health = vital_type {
        if new_value.get() <= 0.0 {
            kill_entity(entity, world);
        }
    }
}

/// Makes entities pass out when they reach 0 energy.
pub fn sleep_on_zero_energy(
    notification: &Notification<VitalChangedNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    let vital_type = notification.notification_type.vital_type;
    let new_value = &notification.notification_type.new_value;

    if let VitalType::Energy = vital_type {
        if new_value.get() <= 0.0 {
            interrupt_entity(entity, world);
            queue_action_first(
                world,
                entity,
                Box::new(SleepAction {
                    ticks_slept: 0,
                    notification_sender: ActionNotificationSender::new(),
                }),
            );
        }
    }
}
