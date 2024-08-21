use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BasicTokens, BeforeActionNotification, DynamicMessage, DynamicMessageLocation,
    MessageCategory, MessageDelay, MessageFormat, SurroundingsMessageCategory,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const SAY_VERB_NAME: &str = "say";
const SAY_FORMAT: &str = "say <>";
const TEXT_CAPTURE: &str = "text";

static SAY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(\"|say )(?P<text>.*)").unwrap());

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

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Makes an entity say something.
#[derive(Debug)]
pub struct SayAction {
    pub text: String,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SayAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let text = &self.text;

        ActionResult::builder()
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Speech),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${performing_entity.Name} ${performing_entity.you:say/says}, \"${text}\"",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string("text".into(), text.clone()),
                ),
                world,
            )
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
