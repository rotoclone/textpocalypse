use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;

use crate::{
    command_format::{literal_part, CommandFormat},
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, GameMessage, HelpDescription, VerifyActionNotification,
    World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static HELP_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(literal_part("help")));

pub struct HelpParser;

impl InputParser for HelpParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        HELP_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(HelpAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![HELP_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Shows an entity the help screen.
#[derive(Debug)]
struct HelpAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for HelpAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let message = GameMessage::Help(HelpDescription::for_entity(performing_entity, world));

        ActionResult::builder()
            .with_game_message(performing_entity, message)
            .build_complete_no_tick(true)
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
