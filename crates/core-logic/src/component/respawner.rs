use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    action::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult, LookAction},
    command_format::{one_of_literal_part, CommandFormat},
    component::VerifyResult,
    find_spawn_room,
    input_parser::{CommandTarget, InputParseError, InputParser},
    move_entity,
    notification::Notification,
    ActionTag, BasicTokens, BeforeActionNotification, DynamicMessage, DynamicMessageLocation,
    InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    ActionEndNotification, ActionQueue, AfterActionPerformNotification, ParseCustomInput, Vitals,
};

static RESPAWN_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(one_of_literal_part(nonempty!["respawn", "live"])));

struct RespawnParser;

impl InputParser for RespawnParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        RESPAWN_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(RespawnAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![RESPAWN_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct RespawnAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for RespawnAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        world.entity_mut(performing_entity).insert(Vitals::new());

        let spawn_room = find_spawn_room(world);

        move_entity(performing_entity, spawn_room, world);

        ActionResult::builder()
            .with_message(
                performing_entity,
                "You start to feel more corporeal...".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Long,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Movement),
                    MessageDelay::Short,
                    MessageFormat::new("${entity.Name} appears.")
                        .expect("message format should be valid"),
                    BasicTokens::new().with_entity("entity".into(), performing_entity),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop respawning.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        [].into()
    }

    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_before_notification(notification_type, self, world);
    }

    fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        world: &mut World,
    ) -> Vec<VerifyResult> {
        self.notification_sender
            .send_verify_notification(notification_type, self, world)
    }

    fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_perform_notification(notification_type, self, world);
    }

    fn send_end_notification(&self, notification_type: ActionEndNotification, world: &mut World) {
        self.notification_sender
            .send_end_notification(notification_type, self, world);
    }
}

/// A component that provides the respawn action.
#[derive(Component)]
pub struct Respawner;

impl ParseCustomInput for Respawner {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(RespawnParser)]
    }
}

/// Notification handler that queues up a look action after an entity respawns, so they can see where they ended up.
pub fn look_after_respawn(
    notification: &Notification<AfterActionPerformNotification, RespawnAction>,
    world: &mut World,
) {
    if !notification.notification_type.action_complete
        || !notification.notification_type.action_successful
    {
        return;
    }

    let performing_entity = notification.notification_type.performing_entity;
    if let Some(target) = CommandTarget::Here.find_target_entity(performing_entity, world) {
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(LookAction {
                target,
                detailed: false,
                notification_sender: ActionNotificationSender::new(),
            }),
        );
    }
}
