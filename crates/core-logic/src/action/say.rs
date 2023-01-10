use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Location},
    get_reference_name,
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, MessageDelay, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const SAY_VERB_NAME: &str = "say";
const SAY_FORMAT: &str = "say <>";
const TEXT_CAPTURE: &str = "text";

lazy_static! {
    static ref SAY_PATTERN: Regex = Regex::new("^(\"|say )(?P<text>.*)").unwrap();
}

pub struct SayParser;

impl InputParser for SayParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = SAY_PATTERN.captures(input) {
            if let Some(text_match) = captures.name(TEXT_CAPTURE) {
                return Ok(Box::new(SayAction {
                    text: text_match.as_str().to_string(),
                    notification_sender: ActionNotificationSender::new(),
                }));
            } else {
                return Err(InputParseError::CommandParseError {
                    verb: SAY_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SAY_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct SayAction {
    pub text: String,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SayAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let speaker_name = get_reference_name(performing_entity, None, world);
        let text = &self.text;

        let mut result_builder = ActionResult::builder().with_message(
            performing_entity,
            format!("You say, \"{text}\""),
            MessageDelay::Short,
        );

        if let Some(location) = world.get::<Location>(performing_entity) {
            result_builder = result_builder.with_message_for_other_entities_in_location(
                performing_entity,
                location.id,
                format!("{speaker_name} says, \"{text}\""),
                MessageDelay::Short,
                world,
            );
        }

        result_builder.build_complete_no_tick(true)
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

    fn send_after_notification(
        &self,
        notification_type: AfterActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_notification(notification_type, self, world);
    }
}
