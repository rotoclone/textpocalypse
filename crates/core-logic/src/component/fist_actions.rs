use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    input_parser::InputParser, Action, ActionEndNotification, ActionInterruptResult,
    ActionNotificationSender, ActionResult, AfterActionPerformNotification,
    BeforeActionNotification, CommandParseError, CommandTarget, Description, InputParseError,
    InternalMessageCategory, MessageCategory, MessageDelay, ParseCustomInput,
    VerifyActionNotification, VerifyResult, Vitals,
};

/// A component that provides special attack actions for fists.
#[derive(Component)]
pub struct FistActions;

impl ParseCustomInput for FistActions {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(UppercutParser)]
    }
}

const UPPERCUT_VERB_NAME: &str = "uppercut";
const UPPERCUT_FORMAT: &str = "uppercut <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref UPPERCUT_PATTERN: Regex = Regex::new("^(uppercut) (?P<name>.*)").unwrap();
}

struct UppercutParser;

impl InputParser for UppercutParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = UPPERCUT_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Vitals>(target_entity).is_some() {
                        // target exists and is attackable
                        return Ok(Box::new(UppercutAction {
                            target: target_entity,
                            notification_sender: ActionNotificationSender::new(),
                        }));
                    }
                    let target_name =
                        Description::get_reference_name(target_entity, Some(source_entity), world);
                    return Err(InputParseError::CommandParseError {
                        verb: UPPERCUT_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!("You can't attack {target_name}.")),
                    });
                }
                return Err(InputParseError::CommandParseError {
                    verb: UPPERCUT_VERB_NAME.to_string(),
                    error: CommandParseError::TargetNotFound(target),
                });
            } else {
                //TODO auto-target if entity is in combat with 1 other entity
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![UPPERCUT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct UppercutAction {
    target: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for UppercutAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        //TODO
        ActionResult::message(
            performing_entity,
            "U do a cool uppercut, wow".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
            true,
        )
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop uppercutting.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
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
