use bevy_ecs::prelude::*;

use crate::{
    send_message_on_channel, ConstrainedValue, GameMessage, InterruptedEntities, MessageDelay,
    Time, ValueChangeDescription, ValueType,
};

use super::MessageChannel;

const SATIETY_LOSS_PER_TICK: f32 = 0.005; // loss of 100 satiety in ~3 days
const HYDRATION_LOSS_PER_TICK: f32 = 0.008; // loss of 100 hydration in ~2 days
const ENERGY_LOSS_PER_TICK: f32 = 0.015; // loss of 100 energy in ~1 day

const STARVATION_DAMAGE_PER_TICK: f32 = 5.0;
const THIRST_DAMAGE_PER_TICK: f32 = 5.0;

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

/// Decreases vitals over time.
pub fn vitals_system(
    mut query: Query<(Entity, &mut Vitals, Option<&MessageChannel>)>,
    mut interrupted_entities: ResMut<InterruptedEntities>,
    time: Res<Time>,
) {
    for (entity, mut vitals, channel) in query.iter_mut() {
        if vitals.satiety.get() <= 0.0 {
            apply_damage(
                entity,
                STARVATION_DAMAGE_PER_TICK,
                &mut vitals,
                "You're starving to death!".to_string(),
                channel,
                &time,
                &mut interrupted_entities,
            );
        }

        if vitals.hydration.get() <= 0.0 {
            apply_damage(
                entity,
                THIRST_DAMAGE_PER_TICK,
                &mut vitals,
                "You're dying of thirst!".to_string(),
                channel,
                &time,
                &mut interrupted_entities,
            );
        }

        apply_value_reduction(
            &mut vitals.satiety,
            ValueType::Satiety,
            SATIETY_LOSS_PER_TICK,
            &HUNGER_MESSAGES,
            channel,
            &time,
        );

        apply_value_reduction(
            &mut vitals.hydration,
            ValueType::Hydration,
            HYDRATION_LOSS_PER_TICK,
            &THIRST_MESSAGES,
            channel,
            &time,
        );

        apply_value_reduction(
            &mut vitals.energy,
            ValueType::Energy,
            ENERGY_LOSS_PER_TICK,
            &TIREDNESS_MESSAGES,
            channel,
            &time,
        );
    }
}

/// Reduces the provided entity's health by the provided amount.
///
/// The provided `Vitals` and `MessageChannel` should belong to the provided entity.
pub fn apply_damage(
    entity: Entity,
    amount: f32,
    vitals: &mut Vitals,
    message: String,
    channel: Option<&MessageChannel>,
    time: &Time,
    interrupted_entities: &mut InterruptedEntities,
) {
    let old_value = vitals.health.clone();
    vitals.health.subtract(amount);
    if let Some(channel) = channel {
        let message = ValueChangeDescription {
            message,
            value_type: ValueType::Health,
            old_value,
            new_value: vitals.health.clone(),
        };
        send_message_on_channel(
            channel,
            GameMessage::ValueChange(message, MessageDelay::Short),
            time.clone(),
        );
    }
    interrupted_entities.0.insert(entity);

    if vitals.health.get() <= 0.0 {
        //TODO make entity actually be dead
        if let Some(channel) = channel {
            send_message_on_channel(
                channel,
                GameMessage::Message("Ur dead".to_string(), MessageDelay::Long),
                time.clone(),
            );
        }
    }
}

/// Reduces the provided value by the provided amount, and sends a message on the provided channel if the value passed one of the defined thresholds.
///
/// The provided messages should be ordered from highest fraction to lowest.
pub fn apply_value_reduction(
    value: &mut ConstrainedValue<f32>,
    value_type: ValueType,
    amount: f32,
    messages: &[(f32, &str)],
    channel: Option<&MessageChannel>,
    time: &Time,
) {
    let old_value = value.clone();
    let old_fraction = old_value.get() / old_value.get_max();

    value.subtract(amount);
    let new_fraction = value.get() / value.get_max();

    if let Some(channel) = channel {
        for (fraction, message) in messages.iter().rev() {
            if old_fraction > *fraction && new_fraction <= *fraction {
                send_message_on_channel(
                    channel,
                    GameMessage::ValueChange(
                        ValueChangeDescription {
                            message: message.to_string(),
                            value_type,
                            old_value,
                            new_value: value.clone(),
                        },
                        MessageDelay::Short,
                    ),
                    time.clone(),
                );
                break;
            }
        }
    }
}
