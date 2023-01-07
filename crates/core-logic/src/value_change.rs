use bevy_ecs::prelude::*;

use crate::{
    component::Vitals,
    notification::{Notification, NotificationType},
    send_message, ConstrainedValue, GameMessage, MessageDelay, ValueChangeDescription,
};

/// A change to a value.
pub struct ValueChange {
    /// The entity to change the value on.
    pub entity: Entity,
    /// The type of value to change.
    pub value_type: ValueType,
    /// The manner by which the value should be changed.
    pub operation: ValueChangeOperation,
    /// The amount by which the value should be changed.
    pub amount: f32,
    /// The accompanying message, if the entity should be made aware of the change.
    pub message: Option<String>,
}

/// A type of change to a value.
pub enum ValueChangeOperation {
    /// A value should be added to the value.
    Add,
    /// A value should be subtracted from the value.
    Subtract,
    /// The value should be multiplied by a value.
    Multiply,
    /// The value should be set to a value.
    Set,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    Health,
    Satiety,
    Hydration,
    Energy,
}

/// A notification for after a value has been changed.
#[derive(Debug)]
pub struct ValueChangedNotification {
    /// The entity the value was changed on.
    pub entity: Entity,
    /// The type of the changed value.
    pub value_type: ValueType,
    /// The value before the change.
    pub old_value: ConstrainedValue<f32>,
    /// The value after the change.
    pub new_value: ConstrainedValue<f32>,
}

impl NotificationType for ValueChangedNotification {}

impl ValueChange {
    /// Applies the value change.
    pub fn apply(self, world: &mut World) {
        let vitals = world.get_mut::<Vitals>(self.entity);
        if let Some(mut vitals) = vitals {
            let value = match self.value_type {
                ValueType::Health => &mut vitals.health,
                ValueType::Satiety => &mut vitals.satiety,
                ValueType::Hydration => &mut vitals.hydration,
                ValueType::Energy => &mut vitals.energy,
            };
            let old_value = value.clone();

            match self.operation {
                ValueChangeOperation::Add => value.add(self.amount),
                ValueChangeOperation::Subtract => value.subtract(self.amount),
                ValueChangeOperation::Multiply => value.multiply(self.amount),
                ValueChangeOperation::Set => value.set(self.amount),
            }

            let new_value = value.clone();

            if let Some(message) = self.message {
                let message = ValueChangeDescription {
                    message,
                    value_type: self.value_type,
                    old_value: old_value.clone(),
                    new_value: new_value.clone(),
                };
                send_message(
                    world,
                    self.entity,
                    GameMessage::ValueChange(message, MessageDelay::Short),
                );
            }

            Notification {
                notification_type: ValueChangedNotification {
                    entity: self.entity,
                    value_type: self.value_type,
                    old_value,
                    new_value,
                },
                contents: &(),
            }
            .send(world);
        }
    }
}
