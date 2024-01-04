use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{ThirdPersonMessage, ThirdPersonMessageLocation},
    checks::{CheckModifiers, VsCheckParams, VsParticipant},
    component::{
        ActionEndNotification, AfterActionPerformNotification, Attribute, CombatState, Stats,
    },
    get_reference_name,
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
    notification::{Notification, VerifyResult},
    BeforeActionNotification, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const DECREASE_RANGE_VERB_NAME: &str = "approach";
const INCREASE_RANGE_VERB_NAME: &str = "move away";
const DECREASE_RANGE_FORMAT: &str = "approach <>";
const INCREASE_RANGE_FORMAT: &str = "move away from <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref DECREASE_RANGE_PATTERN: Regex = Regex::new(
        "^(advance|advance toward|decrease range to|dr|move toward|approach)( (?P<name>.*))?"
    )
    .unwrap();
    static ref INCREASE_RANGE_PATTERN: Regex = Regex::new(
        "^(fall back|fall back from|increase range to|ir|move away from)( (?P<name>.*))?"
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

        let (captures, verb_name, direction) =
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
                        direction,
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
                direction,
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
    pub direction: RangeChangeDirection,
    pub notification_sender: ActionNotificationSender<Self>,
}

/// The direction to change range in.
#[derive(Debug, Clone, Copy)]
pub enum RangeChangeDirection {
    /// Make the range shorter.
    Decrease,
    /// Make the range longer.
    Increase,
}

impl Action for ChangeRangeAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = get_reference_name(target, Some(performing_entity), world);

        let (check_result, _) = Stats::check_vs(
            VsParticipant {
                entity: performing_entity,
                stat: Attribute::Agility.into(),
                modifiers: CheckModifiers::none(),
            },
            VsParticipant {
                entity: target,
                stat: Attribute::Agility.into(),
                modifiers: CheckModifiers::none(),
            },
            VsCheckParams::second_wins_ties(),
            world,
        );

        if !check_result.succeeded() {
            let movement_phrase = match self.direction {
                RangeChangeDirection::Decrease => "get closer to",
                RangeChangeDirection::Increase => "get farther away from",
            };
            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    format!("You look for an opening, but don't manage to {movement_phrase} {target_name}."),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
                .with_third_person_message(
                    Some(performing_entity),
                    ThirdPersonMessageLocation::SourceEntity,
                    ThirdPersonMessage::new(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .add_name(performing_entity)
                    .add_string(format!(" tries to {movement_phrase} "))
                    .add_name(target)
                    .add_string(", but can't manage to."),
                    world,
                )
                .build_complete_should_tick(false);
        }

        // actually change the range
        let current_range = *CombatState::get_entities_in_combat_with(performing_entity, world)
            .get(&target)
            .expect("performing entity should be in combat with target");
        let new_range = match self.direction {
            RangeChangeDirection::Decrease => current_range
                .decreased()
                .expect("range should not already be shortest"),
            RangeChangeDirection::Increase => current_range
                .increased()
                .expect("range should not already be farthest"),
        };
        CombatState::set_in_combat(performing_entity, target, new_range, world);

        let (movement_phrase_second_person, movement_phrase_third_person) = match self.direction {
            RangeChangeDirection::Decrease => (
                "run forward, getting closer to",
                "runs forward, getting closer to",
            ),
            RangeChangeDirection::Increase => (
                "jump backward, getting farther away from",
                "jumps backward, getting farther away from",
            ),
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {movement_phrase_second_person} {target_name}. You're now at {new_range} range."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                )
                .only_send_to(target)
                .add_name(performing_entity)
                .add_string(format!(" {movement_phrase_third_person} "))
                .add_name(target)
                .add_string(". ")
                .add_personal_subject_pronoun(performing_entity, true)
                .add_string(" ")
                .add_to_be_form(performing_entity)
                .add_string(format!(" now at {new_range} range.")),
                world,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                )
                .do_not_send_to(target)
                .add_name(performing_entity)
                .add_string(format!(" {movement_phrase_third_person} "))
                .add_name(target)
                .add_string("."),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop repositioning.".to_string(),
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

/// Verifies that the range can actually be changed in the requested direction.
pub fn verify_range_can_be_changed(
    notification: &Notification<VerifyActionNotification, ChangeRangeAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let target = notification.contents.target;
    let direction = notification.contents.direction;
    let target_name = get_reference_name(target, Some(performing_entity), world);

    if let Some(range) =
        CombatState::get_entities_in_combat_with(performing_entity, world).get(&target)
    {
        let valid = match direction {
            RangeChangeDirection::Decrease => range.decreased().is_some(),
            RangeChangeDirection::Increase => range.increased().is_some(),
        };

        if !valid {
            let range_description = match direction {
                RangeChangeDirection::Decrease => "as close to",
                RangeChangeDirection::Increase => "as far away from",
            };
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!(
                    "You're already {range_description} {target_name} as you can get."
                )),
            );
        }
    } else {
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("You're not in combat with {target_name}.")),
        );
    }

    VerifyResult::valid()
}
