use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{
        clear_action_queue, ActionEndNotification, ActionQueue, AfterActionPerformNotification,
    },
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const STOP_FORMAT: &str = "stop";

lazy_static! {
    static ref STOP_PATTERN: Regex = Regex::new("^(stop|cancel)$").unwrap();
}

pub struct StopParser;

impl InputParser for StopParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if STOP_PATTERN.is_match(input) {
            return Ok(Box::new(StopAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![STOP_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
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
                clear_action_queue(world, performing_entity);
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
