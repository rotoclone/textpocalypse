use bevy_ecs::prelude::*;

use crate::{
    action::{LookAction, SayAction},
    notification::VerifyResult,
    GameMessage,
};

use super::{
    description::DescribeAttributes, AttributeDescriber, AttributeDescription,
    AttributeDetailLevel, VerifyActionNotification,
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

/// Determines whether the provided entity is asleep.
pub fn is_asleep(entity: Entity, world: &World) -> bool {
    world
        .get::<SleepState>(entity)
        .map_or(false, |s| s.is_asleep)
}

/// Prevents an entity from looking at anything while it is asleep.
pub fn prevent_look_while_asleep(
    notification: &VerifyActionNotification<LookAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.performing_entity;

    if is_asleep(performing_entity, world) {
        let message = GameMessage::Error("You can't look while you're asleep.".to_string());
        return VerifyResult::invalid(performing_entity, message);
    }

    VerifyResult::valid()
}

/// Prevents an entity from saying anything while it is asleep.
pub fn prevent_say_while_asleep(
    notification: &VerifyActionNotification<SayAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.performing_entity;

    if is_asleep(performing_entity, world) {
        let message = GameMessage::Error("You can't talk while you're asleep.".to_string());
        return VerifyResult::invalid(performing_entity, message);
    }

    VerifyResult::valid()
}
