use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, GameMessage, VerifyActionNotification, World,
    WornItemsDescription,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const WORN_FORMAT: &str = "worn";

static WORN_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(worn|wearing|clothes|clothing)$").unwrap());

pub struct WornParser;

impl InputParser for WornParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if WORN_PATTERN.is_match(input) {
            return Ok(Box::new(WornAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![WORN_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Shows an entity the items it's wearing.
#[derive(Debug)]
pub struct WornAction {
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for WornAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(desc) = WornItemsDescription::from_entity(performing_entity, world) {
            ActionResult::builder()
                .with_game_message(performing_entity, GameMessage::WornItems(desc))
                .build_complete_no_tick(true)
        } else {
            ActionResult::error(performing_entity, "You have no worn items.".to_string())
        }
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
