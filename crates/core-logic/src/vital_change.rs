use std::fmt::Display;

use bevy_ecs::prelude::*;

use crate::{
    component::Vitals,
    notification::{Notification, NotificationType},
    send_message, ConstrainedValue, DynamicMessage, DynamicMessageLocation, GameMessage,
    MessageCategory, MessageDecoration, MessageDelay, MessageTokens, VitalChangeDescription,
    VitalChangeShortDescription,
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
    Direct {
        entity: Entity,
        message: String,
        category: MessageCategory,
    },
    /// A dynamic message
    Dynamic(DynamicMessage<T>),
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

impl Display for VitalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            VitalType::Health => "health",
            VitalType::Satiety => "satiety",
            VitalType::Hydration => "hydration",
            VitalType::Energy => "energy",
        };

        string.fmt(f)
    }
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
            let old_value = *value;

            match self.operation {
                ValueChangeOperation::Add => value.add(self.amount),
                ValueChangeOperation::Subtract => value.subtract(self.amount),
                ValueChangeOperation::Multiply => value.multiply(self.amount),
                ValueChangeOperation::Set => value.set(self.amount),
            }

            let new_value = *value;

            let description = VitalChangeDescription {
                vital_type: self.vital_type,
                old_value,
                new_value,
            };
            let short_description =
                VitalChangeShortDescription::from_vital_change_description(&description);

            for (message_params, visualization_type) in self.message_params {
                let decoration = match visualization_type {
                    VitalChangeVisualizationType::Full => {
                        MessageDecoration::VitalChange(description.clone())
                    }
                    VitalChangeVisualizationType::Abbreviated => {
                        MessageDecoration::ShortVitalChange(short_description.clone())
                    }
                };

                match message_params {
                    VitalChangeMessageParams::Direct {
                        entity,
                        message,
                        category,
                    } => {
                        send_message(
                            world,
                            entity,
                            GameMessage::Message {
                                content: message,
                                category,
                                delay: MessageDelay::Short,
                                decorations: vec![decoration],
                            },
                        );
                    }
                    VitalChangeMessageParams::Dynamic(dynamic_message) => {
                        dynamic_message.with_decoration(decoration).send(
                            Some(self.entity),
                            DynamicMessageLocation::SourceEntity,
                            world,
                        );
                    }
                };
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
