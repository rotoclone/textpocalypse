use bevy_ecs::prelude::*;

use crate::{
    component::Vitals,
    notification::{Notification, NotificationType},
    send_message, ConstrainedValue, GameMessage, MessageDelay, MessageTokens, ThirdPersonMessage,
    VitalChangeDescription, VitalChangeShortDescription,
};

/// A change to a vital value.
pub struct VitalChange<T: MessageTokens> {
    /// The entity to change the value on.
    pub entity: Entity,
    /// The type of vital to change.
    pub vital_type: VitalType,
    /// The manner by which the value should be changed.
    pub operation: ValueChangeOperation,
    /// The amount by which the value should be changed.
    pub amount: f32,
    /// The accompanying messages to send for the change.
    pub message_params: Vec<(VitalChangeMessageParams<T>, VitalChangeVisualizationType)>,
}

/// Describes a message to send about a vital change.
pub enum VitalChangeMessageParams<T: MessageTokens> {
    /// A message sent directly to an entity
    Direct(Entity, String),
    /// A third person message
    ThirdPerson(ThirdPersonMessage<T>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// The value after the change.f
    pub new_value: ConstrainedValue<f32>,
}

impl NotificationType for VitalChangedNotification {}

impl<T: MessageTokens> VitalChange<T> {
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

            for (message_params, visualization_type) in self.message_params {
                let description = match message_params {
                    VitalChangeMessageParams::Direct(entity, message) => VitalChangeDescription {
                        message,
                        vital_type: self.vital_type,
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    },
                    VitalChangeMessageParams::ThirdPerson(message) => todo!(),
                };
                let message = match visualization_type {
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
