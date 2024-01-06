use bevy_ecs::prelude::*;

use crate::{
    component::Vitals,
    notification::{Notification, NotificationType},
    send_message, ConstrainedValue, GameMessage, MessageDelay, VitalChangeDescription,
};

/// A change to a vital value.
pub struct VitalChange {
    /// The entity to change the value on.
    pub entity: Entity,
    /// The type of vital to change.
    pub vital_type: VitalType,
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
pub enum VitalType {
    Health,
    Satiety,
    Hydration,
    Energy,
}

/// A notification for after a vital value has been changed.
#[derive(Debug)]
pub struct VitalChangedNotification {
    /// The entity the value was changed on.
    pub entity: Entity,
    /// The type of the changed vital.
    pub vital_type: VitalType,
    /// The value before the change.
    pub old_value: ConstrainedValue<f32>,
    /// The value after the change.
    pub new_value: ConstrainedValue<f32>,
}

impl NotificationType for VitalChangedNotification {}

impl VitalChange {
    /// Applies the value change.
    pub fn apply(self, world: &mut World) {
        let vitals = world.get_mut::<Vitals>(self.entity);
        if let Some(mut vitals) = vitals {
            let value = match self.vital_type {
                VitalType::Health => &mut vitals.health,
                VitalType::Satiety => &mut vitals.satiety,
                VitalType::Hydration => &mut vitals.hydration,
                VitalType::Energy => &mut vitals.energy,
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
                let message = VitalChangeDescription {
                    message,
                    vital_type: self.vital_type,
                    old_value: old_value.clone(),
                    new_value: new_value.clone(),
                };
                send_message(
                    world,
                    self.entity,
                    GameMessage::VitalChange(message, MessageDelay::Short),
                );
            }

            //TODO add ability to specify other entities to send notifications to
            Notification {
                notification_type: VitalChangedNotification {
                    entity: self.entity,
                    vital_type: self.vital_type,
                    old_value,
                    new_value,
                },
                contents: &(),
            }
            .send(world);
        }
    }
}
