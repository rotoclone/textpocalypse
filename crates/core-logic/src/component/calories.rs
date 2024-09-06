use bevy_ecs::prelude::*;

use crate::{
    action::EatAction,
    notification::Notification,
    vital_change::{
        ValueChangeOperation, VitalChange, VitalChangeMessageParams, VitalChangeVisualizationType,
        VitalType,
    },
    AttributeDescription, InternalMessageCategory, MessageCategory, NoTokens,
};

use super::{
    AfterActionPerformNotification, AttributeDescriber, AttributeDetailLevel, DescribeAttributes,
};

/// The amount of satiety gained per calorie eaten.
const SATIETY_GAIN_PER_CALORIE: f32 = 0.01;

/// Describes how many calories an entity contains.
#[derive(Component)]
pub struct Calories(pub u16);

/// Increases satiety when an entity is eaten based on its calories.
pub fn increase_satiety_on_eat(
    notification: &Notification<AfterActionPerformNotification, EatAction>,
    world: &mut World,
) {
    if notification.notification_type.action_complete
        && notification.notification_type.action_successful
    {
        if let Some(calories) = world.get::<Calories>(notification.contents.target) {
            VitalChange::<NoTokens> {
                entity: notification.notification_type.performing_entity,
                vital_type: VitalType::Satiety,
                operation: ValueChangeOperation::Add,
                amount: f32::from(calories.0) * SATIETY_GAIN_PER_CALORIE,
                message_params: vec![(
                    VitalChangeMessageParams::Direct {
                        entity: notification.notification_type.performing_entity,
                        message: "That hit the spot!".to_string(),
                        category: MessageCategory::Internal(InternalMessageCategory::Misc),
                    },
                    VitalChangeVisualizationType::Full,
                )],
            }
            .apply(world);
        }
    }
}
