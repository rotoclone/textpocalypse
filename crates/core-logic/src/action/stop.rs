use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{literal_part, one_of_part, CommandFormat},
    component::{ActionEndNotification, ActionQueue, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static STOP_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("stop"),
        literal_part("cancel")
    ]))
});

pub struct StopParser;

impl InputParser for StopParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        STOP_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(StopAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![STOP_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Makes an entity stop its current action.
#[derive(Debug)]
pub struct StopAction {
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for StopAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(queue) = world.get::<ActionQueue>(performing_entity) {
            if !queue.is_empty() {
                ActionQueue::clear(world, performing_entity);
                return ActionResult::builder()
                    .with_message(
                        performing_entity,
                        "You stop what you were doing.".to_string(),
                        MessageCategory::Internal(InternalMessageCategory::Misc),
                        MessageDelay::None,
                    )
                    .build_complete_no_tick(true);
            }
        }

        ActionResult::builder()
            .with_message(
                performing_entity,
                "You aren't doing anything.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Misc),
                MessageDelay::None,
            )
            .build_complete_no_tick(false)
    }

    fn interrupt(&self, _: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::none()
    }

    fn may_require_tick(&self) -> bool {
        false
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
    ) -> VerifyResult {
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
