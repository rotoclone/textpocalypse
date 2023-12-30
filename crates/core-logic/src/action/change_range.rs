use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, CombatState},
    get_reference_name,
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const DECREASE_RANGE_VERB_NAME: &str = "advance";
const INCREASE_RANGE_VERB_NAME: &str = "retreat";
const DECREASE_RANGE_FORMAT: &str = "advance toward <>";
const INCREASE_RANGE_FORMAT: &str = "retreat from <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref DECREASE_RANGE_PATTERN: Regex = Regex::new(
        "^(advance|advance toward|charge|charge toward|decrease range to|dr|move toward) (?P<name>.*)"
    )
    .unwrap();
    static ref INCREASE_RANGE_PATTERN: Regex = Regex::new(
        "^(retreat|retreat from|fall back|fall back from|increase range to|ir|move away from) (?P<name>.*)"
    )
    .unwrap();
}

pub struct ChangeRangeParser;

impl InputParser for ChangeRangeParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let valid_targets = CombatState::get_entities_in_combat_with(source_entity, world);

        let (captures, verb_name, change_direction) =
            if let Some(captures) = DECREASE_RANGE_PATTERN.captures(input) {
                (
                    captures,
                    DECREASE_RANGE_VERB_NAME,
                    RangeChangeDirection::Decrease,
                )
            } else if let Some(captures) = INCREASE_RANGE_PATTERN.captures(input) {
                (
                    captures,
                    INCREASE_RANGE_VERB_NAME,
                    RangeChangeDirection::Increase,
                )
            } else {
                return Err(InputParseError::UnknownCommand);
            };

        if valid_targets.is_empty() {
            return Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::Other("You're not in combat with anyone.".to_string()),
            });
        }

        if let Some(target_match) = captures.name(NAME_CAPTURE) {
            let command_target = CommandTarget::parse(target_match.as_str());
            if let Some(target) = command_target.find_target_entity(source_entity, world) {
                if valid_targets.contains_key(&target) {
                    // in combat with target
                    Ok(Box::new(ChangeRangeAction {
                        target,
                        change_direction,
                        notification_sender: ActionNotificationSender::new(),
                    }))
                } else {
                    // not in combat with target
                    let target_name = get_reference_name(target, Some(source_entity), world);
                    Err(InputParseError::CommandParseError {
                        verb: verb_name.to_string(),
                        error: CommandParseError::Other(format!(
                            "You're not in combat with {target_name}."
                        )),
                    })
                }
            } else {
                Err(InputParseError::CommandParseError {
                    verb: verb_name.to_string(),
                    error: CommandParseError::TargetNotFound(command_target),
                })
            }
        } else if valid_targets.len() == 1 {
            // the source entity is only in combat with one other entity, so auto-choose target
            Ok(Box::new(ChangeRangeAction {
                // unwrap is safe here because we just checked if the length is 1
                target: *valid_targets.keys().next().unwrap(),
                change_direction,
                notification_sender: ActionNotificationSender::new(),
            }))
        } else {
            Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::MissingTarget,
            })
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            DECREASE_RANGE_FORMAT.to_string(),
            INCREASE_RANGE_FORMAT.to_string(),
        ]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        if CombatState::get_entities_in_combat_with(pov_entity, world).contains_key(&entity) {
            Some(vec![
                DECREASE_RANGE_FORMAT.to_string(),
                INCREASE_RANGE_FORMAT.to_string(),
            ])
        } else {
            None
        }
    }
}

/// Makes an entity attempt to change the range to another entity it's in combat with.
#[derive(Debug)]
pub struct ChangeRangeAction {
    pub target: Entity,
    pub change_direction: RangeChangeDirection,
    pub notification_sender: ActionNotificationSender<Self>,
}

/// The direction to change range in.
#[derive(Debug)]
pub enum RangeChangeDirection {
    /// Make the range shorter.
    Decrease,
    /// Make the range longer.
    Increase,
}

impl Action for ChangeRangeAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        todo!() //TODO
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop changing range.".to_string(),
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
