use std::collections::HashSet;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, OpenState},
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    ActionTag, BasicTokens, BeforeActionNotification, Description, InternalMessageCategory,
    MessageCategory, MessageDelay, MessageFormat, SurroundingsMessageCategory,
    VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ThirdPersonMessage,
    ThirdPersonMessageLocation,
};

const OPEN_VERB_NAME: &str = "open";
const CLOSE_VERB_NAME: &str = "close";
const OPEN_FORMAT: &str = "open <>";
const CLOSE_FORMAT: &str = "close <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref OPEN_PATTERN: Regex = Regex::new("^open (the )?(?P<name>.*)").unwrap();
    static ref CLOSE_PATTERN: Regex = Regex::new("^close (the )?(?P<name>.*)").unwrap();
}

pub struct OpenParser;

impl InputParser for OpenParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (captures, verb_name, should_be_open) =
            if let Some(captures) = OPEN_PATTERN.captures(input) {
                (captures, OPEN_VERB_NAME, true)
            } else if let Some(captures) = CLOSE_PATTERN.captures(input) {
                (captures, CLOSE_VERB_NAME, false)
            } else {
                return Err(InputParseError::UnknownCommand);
            };

        if let Some(target_match) = captures.name(NAME_CAPTURE) {
            let command_target = CommandTarget::parse(target_match.as_str());
            if let Some(target) = command_target.find_target_entity(source_entity, world) {
                Ok(Box::new(OpenAction {
                    target,
                    should_be_open,
                    notification_sender: ActionNotificationSender::new(),
                }))
            } else {
                Err(InputParseError::CommandParseError {
                    verb: verb_name.to_string(),
                    error: CommandParseError::TargetNotFound(command_target),
                })
            }
        } else {
            Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::MissingTarget,
            })
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![OPEN_FORMAT.to_string(), CLOSE_FORMAT.to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        input_formats_if_has_component::<OpenState>(entity, world, &[OPEN_FORMAT, CLOSE_FORMAT])
    }
}

/// Makes an entity open or close something.
#[derive(Debug)]
pub struct OpenAction {
    pub target: Entity,
    pub should_be_open: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for OpenAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let state = match world.get::<OpenState>(self.target) {
            Some(s) => s,
            None => {
                if self.should_be_open {
                    return ActionResult::error(
                        performing_entity,
                        "You can't open that.".to_string(),
                    );
                } else {
                    return ActionResult::error(
                        performing_entity,
                        "You can't close that.".to_string(),
                    );
                }
            }
        };

        if state.is_open == self.should_be_open {
            if state.is_open {
                return ActionResult::message(
                    performing_entity,
                    "It's already open.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            } else {
                return ActionResult::message(
                    performing_entity,
                    "It's already closed.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            }
        }

        OpenState::set_open(self.target, self.should_be_open, world);

        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);
        let (open_or_close, opens_or_closes) = if self.should_be_open {
            ("open", "opens")
        } else {
            ("close", "closes")
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {open_or_close} {target_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${performing_entity.Name} ${opens_or_closes} ${target.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string("opens_or_closes".into(), opens_or_closes.to_string())
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop opening.".to_string(),
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
