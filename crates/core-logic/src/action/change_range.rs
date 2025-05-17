use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    checks::{CheckModifiers, VsCheckParams, VsParticipant},
    combat_utils::is_valid_attack_target,
    command_format::{
        entity_part_with_validator, literal_part, one_of_part, CommandFormat, CommandParseError,
        CommandPartId, CommandPartValidateError, CommandPartValidateResult, PartValidatorContext,
    },
    component::{
        ActionEndNotification, AfterActionPerformNotification, Attribute, CombatState, Stats,
    },
    input_parser::InputParser,
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification, STANDARD_CHECK_XP,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static DECREASE_RANGE_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("approach"),
        literal_part("advance"),
        literal_part("decrease range"),
        literal_part("dr"),
    ]))
});

static INCREASE_RANGE_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("fall back"),
        literal_part("increase range"),
        literal_part("ir"),
    ]))
});

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));

static DECREASE_RANGE_WITH_TARGET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("approach"),
        literal_part("advance toward"),
        literal_part("advance towards"),
        literal_part("decrease range to"),
        literal_part("dr"),
    ]))
    .then(literal_part(" "))
    .then(entity_part_with_validator(
        TARGET_PART_ID.clone(),
        validate_target,
    ))
});

static INCREASE_RANGE_WITH_TARGET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("fall back"),
        literal_part("fall back from"),
        literal_part("increase range to"),
        literal_part("ir"),
    ]))
    .then(literal_part(" "))
    .then(entity_part_with_validator(
        TARGET_PART_ID.clone(),
        validate_target,
    ))
});

/// Determines whether an entity could be a valid target for a change range command.
fn validate_target(
    context: PartValidatorContext<Entity>,
    world: &World,
) -> CommandPartValidateResult {
    if context.parsed_value == context.performing_entity {
        return CommandPartValidateResult::Invalid(CommandPartValidateError {
            details: Some(
                "You can't get closer or farther from yourself. At least not in a physical sense."
                    .to_string(),
            ),
        });
    }

    if is_valid_attack_target(context.parsed_value, world) {
        CommandPartValidateResult::Valid
    } else {
        let target_name = Description::get_reference_name(
            context.parsed_value,
            Some(context.performing_entity),
            world,
        );
        let message = format!("You can't change your range to {target_name}.");
        CommandPartValidateResult::Invalid(CommandPartValidateError {
            details: Some(message),
        })
    }
}

pub struct ChangeRangeParser;

impl InputParser for ChangeRangeParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let valid_targets = CombatState::get_entities_in_combat_with(source_entity, world);

        if valid_targets.is_empty() {
            return Err(CommandParseError::Other(
                "You're not in combat with anyone.".to_string(),
            ));
        }

        let (direction, target) = if valid_targets.len() > 1 {
            parse_with_required_target(input, source_entity, world)?
        } else {
            // already checked if there are no valid targets above, so this must mean there is only one possible target
            let (direction, provided_target) =
                parse_with_optional_target(input, source_entity, world)?;
            let target = match provided_target {
                Some(t) => t,
                // the source entity is only in combat with one other entity, so auto-choose target
                // unwrap is safe because we should only get here if the length is 1
                None => *valid_targets.keys().next().unwrap(),
            };
            (direction, target)
        };

        if valid_targets.contains_key(&target) {
            Ok(Box::new(ChangeRangeAction {
                direction,
                target,
                notification_sender: ActionNotificationSender::new(),
            }))
        } else {
            let target_name = Description::get_reference_name(target, Some(source_entity), world);
            Err(CommandParseError::Other(format!(
                "You're not in combat with {target_name}."
            )))
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            DECREASE_RANGE_FORMAT.get_format_description().to_string(),
            DECREASE_RANGE_WITH_TARGET_FORMAT
                .get_format_description()
                .to_string(),
            INCREASE_RANGE_FORMAT.get_format_description().to_string(),
            INCREASE_RANGE_WITH_TARGET_FORMAT
                .get_format_description()
                .to_string(),
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
                DECREASE_RANGE_WITH_TARGET_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world)
                    .to_string(),
                INCREASE_RANGE_WITH_TARGET_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world)
                    .to_string(),
            ])
        } else {
            None
        }
    }
}

/// Determines which format the command was in and what direction and target were provided.
/// Only checks formats with targets.
fn parse_with_required_target(
    input: &str,
    source_entity: Entity,
    world: &World,
) -> Result<(RangeChangeDirection, Entity), CommandParseError> {
    match DECREASE_RANGE_WITH_TARGET_FORMAT.parse(input, source_entity, world) {
        Ok(parsed) => {
            return Ok((RangeChangeDirection::Decrease, parsed.get(&TARGET_PART_ID)));
        }
        Err(e) => {
            if e.any_parts_matched() {
                return Err(e);
            }
        }
    };

    let parsed = INCREASE_RANGE_WITH_TARGET_FORMAT.parse(input, source_entity, world)?;
    Ok((RangeChangeDirection::Increase, parsed.get(&TARGET_PART_ID)))
}

/// Determines which format the command was in and what direction and target (if any) were provided.
fn parse_with_optional_target(
    input: &str,
    source_entity: Entity,
    world: &World,
) -> Result<(RangeChangeDirection, Option<Entity>), CommandParseError> {
    if let Ok(_) = DECREASE_RANGE_FORMAT.parse(input, source_entity, world) {
        return Ok((RangeChangeDirection::Decrease, None));
    }

    if let Ok(_) = INCREASE_RANGE_FORMAT.parse(input, source_entity, world) {
        return Ok((RangeChangeDirection::Increase, None));
    }

    parse_with_required_target(input, source_entity, world)
        .map(|(direction, target)| (direction, Some(target)))
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
        let target_name = Description::get_reference_name(target, Some(performing_entity), world);

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
            VsCheckParams::second_wins_ties(STANDARD_CHECK_XP),
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
                .with_dynamic_message(
                    Some(performing_entity),
                    DynamicMessageLocation::SourceEntity,
                    DynamicMessage::new_third_person(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                        MessageFormat::new("${performing_entity.Name} tries to ${movement_phrase} ${target.name}, but can't manage to.")
                            .expect("message format should be valid"),
                        BasicTokens::new()
                            .with_entity("performing_entity".into(), performing_entity)
                            .with_string("movement_phrase".into(), movement_phrase.to_string())
                            .with_entity("target".into(), target),
                    ),
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
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${performing_entity.Name} ${movement_phrase} ${target.name}. ${performing_entity.They} ${performing_entity.are/is} now at ${new_range} range.")
                            .expect("message format should be valid"),
                        BasicTokens::new()
                            .with_entity("performing_entity".into(), performing_entity)
                            .with_string("movement_phrase".into(), movement_phrase_third_person.to_string())
                            .with_entity("target".into(), target)
                            .with_string("new_range".into(), new_range.to_string()),
                )
                .only_send_to(target),
                world,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${performing_entity.Name} ${movement_phrase} ${target.name}.")
                            .expect("message format should be valid"),
                        BasicTokens::new()
                            .with_entity("performing_entity".into(), performing_entity)
                            .with_string("movement_phrase".into(), movement_phrase_third_person.to_string())
                            .with_entity("target".into(), target),
                )
                .do_not_send_to(target),
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

    fn get_tags(&self) -> HashSet<ActionTag> {
        [ActionTag::Combat].into()
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
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

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
