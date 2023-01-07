use bevy_ecs::prelude::*;

use super::{
    description::DescribeAttributes, AttributeDescriber, AttributeDescription, AttributeDetailLevel,
};

/// Describes whether an entity is asleep or awake.
#[derive(Component)]
pub struct SleepState {
    /// Whether the entity is asleep.
    pub is_asleep: bool,
}

/// Describes whether the entity is asleep or not.
#[derive(Debug)]
struct SleepStateAttributeDescriber;

impl AttributeDescriber for SleepStateAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(sleep_state) = world.get::<SleepState>(entity) {
            if sleep_state.is_asleep {
                return vec![AttributeDescription::is("asleep".to_string())];
            }
        }

        Vec::new()
    }
}

impl DescribeAttributes for SleepState {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(SleepStateAttributeDescriber)
    }
}
