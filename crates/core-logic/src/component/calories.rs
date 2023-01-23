use bevy_ecs::prelude::*;

use crate::{
    action::EatAction,
    notification::Notification,
    value_change::{ValueChange, ValueChangeOperation},
    AttributeDescription, ValueType,
};

use super::{
    AfterActionPerformNotification, AttributeDescriber, AttributeDetailLevel, DescribeAttributes,
};

/// The amount of satiety gained per calorie eaten.
const SATIETY_GAIN_PER_CALORIE: f32 = 0.01;

/// Describes how many calories an entity contains.
#[derive(Component)]
pub struct Calories(pub u16);

/// Describes the number of calories of an entity.
#[derive(Debug)]
struct CaloriesAttributeDescriber;

impl AttributeDescriber for CaloriesAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if detail_level >= AttributeDetailLevel::Advanced {
            if let Some(calories) = world.get::<Calories>(entity) {
                return vec![AttributeDescription::does(format!(
                    "contains {} calories",
                    calories.0
                ))];
            }
        }

        Vec::new()
    }
}

impl DescribeAttributes for Calories {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(CaloriesAttributeDescriber)
    }
}

/// Increases satiety when an entity is eaten based on its calories.
pub fn increase_satiety_on_eat(
    notification: &Notification<AfterActionPerformNotification, EatAction>,
    world: &mut World,
) {
    if notification.notification_type.action_complete
        && notification.notification_type.action_successful
    {
        if let Some(calories) = world.get::<Calories>(notification.contents.target) {
            ValueChange {
                entity: notification.notification_type.performing_entity,
                value_type: ValueType::Satiety,
                operation: ValueChangeOperation::Add,
                amount: f32::from(calories.0) * SATIETY_GAIN_PER_CALORIE,
                message: Some("That hit the spot!".to_string()),
            }
            .apply(world);
        }
    }
}
