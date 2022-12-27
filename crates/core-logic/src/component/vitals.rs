use bevy_ecs::prelude::*;

use crate::{ConstrainedValue, GameMessage, MessageQueue, ValueChangeDescription, ValueType};

const SATIETY_LOSS_PER_TICK: f32 = 0.005; // loss of 100 satiety in ~3 days
const HYDRATION_LOSS_PER_TICK: f32 = 0.008; // loss of 100 hydration in ~2 days
const ENERGY_LOSS_PER_TICK: f32 = 0.015; // loss of 100 energy in ~1 day

const STARVATION_DAMAGE_PER_TICK: f32 = 5.0;
const THIRST_DAMAGE_PER_TICK: f32 = 5.0;

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
    mut query: Query<(Entity, &mut Vitals)>,
    mut message_queue: ResMut<MessageQueue>,
) {
    for (entity, mut vitals) in query.iter_mut() {
        if vitals.satiety.get() <= 0.0 {
            let old_value = vitals.health.clone();
            vitals.health.subtract(STARVATION_DAMAGE_PER_TICK);
            let message = ValueChangeDescription {
                message: "You're starving!".to_string(),
                value_type: ValueType::Health,
                old_value,
                new_value: vitals.health.clone(),
            };
            message_queue.add(entity, GameMessage::ValueChange(message));
        }

        if vitals.hydration.get() <= 0.0 {
            let old_value = vitals.health.clone();
            vitals.health.subtract(THIRST_DAMAGE_PER_TICK);
            let message = ValueChangeDescription {
                message: "You're dying of thirst!".to_string(),
                value_type: ValueType::Health,
                old_value,
                new_value: vitals.health.clone(),
            };
            message_queue.add(entity, GameMessage::ValueChange(message));
        }

        vitals.satiety.subtract(SATIETY_LOSS_PER_TICK);
        vitals.hydration.subtract(HYDRATION_LOSS_PER_TICK);
        vitals.energy.subtract(ENERGY_LOSS_PER_TICK);
    }
}
