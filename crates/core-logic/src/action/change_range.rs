use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use bevy_ecs::prelude::*;
use core_logic_derive::ActionBoilerplate;
use nonempty::nonempty;

use crate::{
    checks::{CheckModifiers, VsCheckParams, VsParticipant},
    combat_utils::is_valid_attack_target,
    command_format::{
        entity_part_builder, literal_part, one_of_literal_part, CommandFormat,
        CommandFormatParseError, CommandPartId, CommandPartValidateError,
        CommandPartValidateResult, PartValidatorContext,
    },
    component::{Attribute, CombatRange, CombatState, Stats, VerifyResult},
    input_parser::{InputParseError, InputParser},
    message_format::{MessageTokens, TokenName, TokenValue},
    notification::Notification,
    resource::{ActionInteractionContext, ActionInteractionResult},
    ActionTag, BasicTokens, Description, DynamicMessage, DynamicMessageLocation, GameMessage,
    InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification, STANDARD_CHECK_XP,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static DECREASE_RANGE_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "approach",
        "advance",
        "decrease range",
        "dr",
    ]))
});

static INCREASE_RANGE_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "fall back",
        "increase range",
        "ir",
    ]))
});

static TARGET_PART_ID: CommandPartId<Entity> = CommandPartId::new("target");

static DECREASE_RANGE_WITH_TARGET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "approach",
        "advance toward",
        "advance towards",
        "decrease range to",
        "dr",
    ]))
    .then(literal_part(" "))
    .then(
        entity_part_builder(TARGET_PART_ID)
            .with_validator(validate_target)
            .build()
            .with_if_unparsed("who")
            .with_placeholder_for_format_string("target"),
    )
});

static INCREASE_RANGE_WITH_TARGET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "fall back",
        "fall back from",
        "increase range to",
        "ir",
    ]))
    .then(literal_part(" "))
    .then(
        entity_part_builder(TARGET_PART_ID)
            .with_validator(validate_target)
            .build()
            .with_if_unparsed("who")
            .with_placeholder_for_format_string("target"),
    )
});

/// Determines whether an entity could be a valid target for a change range command.
fn validate_target(
    context: &PartValidatorContext<Entity>,
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

//TODO split into multiple parsers
pub struct ChangeRangeParser;

impl InputParser for ChangeRangeParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let valid_targets = CombatState::get_entities_in_combat_with(source_entity, world);

        if valid_targets.is_empty() {
            return Err(InputParseError::PreFormatParse(
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
                // unwrap is safe because we should only get here if the length is 1 due to the `is_empty` and `len() > 1` checks above
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
            Err(InputParseError::PostFormatParse(format!(
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
    ) -> Vec<String> {
        if CombatState::get_entities_in_combat_with(pov_entity, world).contains_key(&entity) {
            vec![
                DECREASE_RANGE_WITH_TARGET_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID, entity, world)
                    .to_string(),
                INCREASE_RANGE_WITH_TARGET_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID, entity, world)
                    .to_string(),
            ]
        } else {
            Vec::new()
        }
    }
}

/// Determines which format the command was in and what direction and target were provided.
/// Only checks formats with targets.
fn parse_with_required_target(
    input: &str,
    source_entity: Entity,
    world: &World,
) -> Result<(RangeChangeDirection, Entity), CommandFormatParseError> {
    match DECREASE_RANGE_WITH_TARGET_FORMAT.parse(input, source_entity, world) {
        Ok(parsed) => {
            return Ok((RangeChangeDirection::Decrease, parsed.get(TARGET_PART_ID)));
        }
        Err(e) => {
            if e.num_parts_matched() > 0 {
                return Err(e);
            }
        }
    };

    let parsed = INCREASE_RANGE_WITH_TARGET_FORMAT.parse(input, source_entity, world)?;
    Ok((RangeChangeDirection::Increase, parsed.get(TARGET_PART_ID)))
}

/// Determines which format the command was in and what direction and target (if any) were provided.
fn parse_with_optional_target(
    input: &str,
    source_entity: Entity,
    world: &World,
) -> Result<(RangeChangeDirection, Option<Entity>), CommandFormatParseError> {
    if DECREASE_RANGE_FORMAT
        .parse(input, source_entity, world)
        .is_ok()
    {
        return Ok((RangeChangeDirection::Decrease, None));
    }

    if INCREASE_RANGE_FORMAT
        .parse(input, source_entity, world)
        .is_ok()
    {
        return Ok((RangeChangeDirection::Increase, None));
    }

    parse_with_required_target(input, source_entity, world)
        .map(|(direction, target)| (direction, Some(target)))
}

/// Makes an entity attempt to change the range to another entity it's in combat with.
#[derive(ActionBoilerplate, Debug)]
pub struct ChangeRangeAction {
    pub target: Entity,
    pub direction: RangeChangeDirection,
    pub notification_sender: ActionNotificationSender<Self>,
}

/// The direction to change range in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeChangeDirection {
    /// Make the range shorter.
    Decrease,
    /// Make the range longer.
    Increase,
}

impl Action for ChangeRangeAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;

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
            let target_name =
                Description::get_reference_name(target, Some(performing_entity), world);
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

        let base_format_string = match self.direction {
            RangeChangeDirection::Decrease => "${performing_entity.Name} ${performing_entity.you:run/runs} forward, getting closer to ${target.name}.",
            RangeChangeDirection::Increase => "${performing_entity.Name} ${performing_entity.you:jump/jumps} backward, getting farther away from ${target.name}."
        };
        let involved_entities_format_suffix =
            "${performing_entity.They} ${performing_entity.are/is} now at ${new_range} range.";

        let involved_entities_format = MessageFormat::new(&format!(
            "{base_format_string} {involved_entities_format_suffix}"
        ))
        .expect("message format should be valid");
        let uninvolved_entities_format =
            MessageFormat::new(base_format_string).expect("message format should be valid");

        change_range(
            performing_entity,
            target,
            self.direction,
            ChangeRangeMessages {
                involved_entities_format,
                uninvolved_entities_format,
            },
            world,
        )
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

    fn get_interaction_target(&self, _: &World) -> Option<Entity> {
        Some(self.target)
    }
}

/// Tokens for messages about changing range.
#[derive(Debug)]
struct ChangeRangeMessageTokens {
    /// The first entity changing range
    performing_entity: Entity,
    /// The second entity changing range
    target: Entity,
    /// The range being changed to
    new_range: CombatRange,
}

impl MessageTokens for ChangeRangeMessageTokens {
    fn get_token_map(&self) -> HashMap<TokenName, TokenValue> {
        [
            (
                "performing_entity".into(),
                TokenValue::Entity(self.performing_entity),
            ),
            ("target".into(), TokenValue::Entity(self.target)),
            (
                "new_range".into(),
                TokenValue::String(self.new_range.to_string()),
            ),
        ]
        .into()
    }
}

/// Messages to send when entities change ranges.
struct ChangeRangeMessages {
    /// The message to send to entities involved in the range change
    involved_entities_format: MessageFormat<ChangeRangeMessageTokens>,
    /// The message to send to entities not involved in the range change
    uninvolved_entities_format: MessageFormat<ChangeRangeMessageTokens>,
}

/// Actually changes the range between `performing_entity` and `target` and returns a result describing it.
fn change_range(
    performing_entity: Entity,
    target: Entity,
    direction: RangeChangeDirection,
    messages: ChangeRangeMessages,
    world: &mut World,
) -> ActionResult {
    let current_range = *CombatState::get_entities_in_combat_with(performing_entity, world)
        .get(&target)
        .expect("performing entity should be in combat with target");
    let new_range = match direction {
        RangeChangeDirection::Decrease => current_range
            .decreased()
            .expect("range should not already be shortest"),
        RangeChangeDirection::Increase => current_range
            .increased()
            .expect("range should not already be farthest"),
    };
    CombatState::set_in_combat(performing_entity, target, new_range, world);

    ActionResult::builder()
        .with_dynamic_message(
            Some(performing_entity),
            DynamicMessageLocation::SourceEntity,
            DynamicMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                messages.involved_entities_format,
                ChangeRangeMessageTokens {
                    performing_entity,
                    target,
                    new_range,
                },
            )
            .only_send_to_entities(&[performing_entity, target]),
            world,
        )
        .with_dynamic_message(
            Some(performing_entity),
            DynamicMessageLocation::SourceEntity,
            DynamicMessage::new_third_person(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                messages.uninvolved_entities_format,
                ChangeRangeMessageTokens {
                    performing_entity,
                    target,
                    new_range,
                },
            )
            .do_not_send_to(target),
            world,
        )
        .build_complete_should_tick(true)
}

/// Handles interactions between 2 change range actions.
///
/// If both actions are trying to change range in the same direction, they'll just happen without any checks.
pub fn change_range_interaction_handler(
    context: ActionInteractionContext<ChangeRangeAction>,
    world: &mut World,
) -> ActionInteractionResult {
    let action_any = context.action_2 as &dyn Any;
    let Some(other_change_range_action) = action_any.downcast_ref::<ChangeRangeAction>() else {
        return ActionInteractionResult::DidNotInteract;
    };

    if other_change_range_action.direction == context.action_1.direction
        && other_change_range_action.target == context.performing_entity_1
        && context.action_1.target == context.performing_entity_2
    {
        let base_format_string = match context.action_1.direction {
            RangeChangeDirection::Decrease => "${performing_entity.Name} and ${target.name} run forward, getting closer to each other.",
            RangeChangeDirection::Increase => "${performing_entity.Name} and ${target.name} jump backward, getting farther away from each other."
        };
        let involved_entities_format_suffix = "You are now at ${new_range} range.";

        let involved_entities_format = MessageFormat::new(&format!(
            "{base_format_string} {involved_entities_format_suffix}"
        ))
        .expect("message format should be valid");
        let uninvolved_entities_format =
            MessageFormat::new(base_format_string).expect("message format should be valid");

        return ActionInteractionResult::Interacted(
            change_range(
                context.performing_entity_1,
                context.performing_entity_2,
                context.action_1.direction,
                ChangeRangeMessages {
                    involved_entities_format,
                    uninvolved_entities_format,
                },
                world,
            ),
            ActionResult::builder().build_complete_should_tick(true),
        );
    }

    ActionInteractionResult::DidNotInteract
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
