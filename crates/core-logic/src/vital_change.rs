use bevy_ecs::prelude::*;

use crate::{
    component::Vitals,
    notification::{Notification, NotificationType},
    send_message, ConstrainedValue, GameMessage, MessageDelay, VitalChangeDescription,
    VitalChangeShortDescription,
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
    /// The accompanying messages to send for the change.
    pub message_params: Vec<VitalChangeMessageParams>,
}

/// Describes a message to send about a vital change.
pub struct VitalChangeMessageParams {
    /// The entity to send the message to
    pub entity: Entity,
    /// The text of the message
    pub message: String,
    /// The type of visualization to accompany the message
    pub visualization_type: VitalChangeVisualizationType,
}

/// The type of visualization to accompany a vital change message.
pub enum VitalChangeVisualizationType {
    /// A full-size bar with the numeric value of the vital.
    Full,
    /// A shorter bar with no numeric value.
    Abbreviated,
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

            for message_params in self.message_params {
                let description = VitalChangeDescription {
                    message: message_params.message,
                    vital_type: self.vital_type,
                    old_value: old_value.clone(),
                    new_value: new_value.clone(),
                };
                let message = match message_params.visualization_type {
                    VitalChangeVisualizationType::Full => {
                        GameMessage::VitalChange(description, MessageDelay::Short)
                    }
                    VitalChangeVisualizationType::Abbreviated => GameMessage::VitalChangeShort(
                        VitalChangeShortDescription::from_vital_change_description(&description),
                        MessageDelay::Short,
                    ),
                };
                send_message(world, message_params.entity, message);
            }

            Notification::send_no_contents(
                VitalChangedNotification {
                    entity: self.entity,
                    vital_type: self.vital_type,
                    old_value,
                    new_value,
                },
                world,
            );
        }
    }
}
