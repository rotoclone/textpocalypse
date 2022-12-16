use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    input_parser::{InputParseError, InputParser},
    notification::Notification,
    BeforeActionNotification, GameMessage, HelpMessage, World,
};

use super::{Action, ActionNotificationSender, ActionResult};

const HELP_FORMAT: &str = "help";

lazy_static! {
    static ref HELP_PATTERN: Regex = Regex::new("^help$").unwrap();
}

pub struct HelpParser;

impl InputParser for HelpParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if HELP_PATTERN.is_match(input) {
            return Ok(Box::new(HelpAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![HELP_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
struct HelpAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for HelpAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let message = GameMessage::Help(HelpMessage::for_entity(performing_entity, world));

        ActionResult::builder()
            .with_game_message(performing_entity, message)
            .build_complete_no_tick()
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
            .send_before_notification(notification_type, &self, world);
    }
}
