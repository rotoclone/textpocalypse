use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use bevy_ecs::prelude::*;
use log::debug;
use nonempty::nonempty;
use regex::Regex;

use crate::{
    command_format::{
        any_text_part_with_validator, entity_part_with_validator, literal_part, one_of_part,
        validate_parsed_value_has_component, CommandFormat, CommandFormatPart, CommandParseError,
        CommandPartId, CommandPartValidateError, CommandPartValidateResult, PartValidatorContext,
    },
    component::{
        ActionEndNotification, AfterActionPerformNotification, FluidContainer, FluidType, Volume,
    },
    input_parser::{input_formats_if_has_component, InputParser},
    notification::VerifyResult,
    resource::get_fluid_name,
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const AMOUNT_CAPTURE: &str = "amount";

static ALL_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^all$").unwrap());
static AMOUNT_WITH_LITERS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(?P<amount>[^ ]*)(L| L| liter| liters)$").unwrap());
static AMOUNT_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^(?P<amount>.*)").unwrap());

static SOURCE_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("source"));
static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static AMOUNT_PART_ID: LazyLock<CommandPartId<String>> =
    LazyLock::new(|| CommandPartId::new("amount"));

static SOURCE_PART: LazyLock<CommandFormatPart> = LazyLock::new(|| {
    entity_part_with_validator(SOURCE_PART_ID.clone(), |context, world| {
        validate_parsed_value_has_component::<FluidContainer>(context, "pour anything from", world)
    })
    .with_if_missing("what")
    .with_placeholder_for_format_string("container")
});
static TARGET_PART: LazyLock<CommandFormatPart> = LazyLock::new(|| {
    entity_part_with_validator(TARGET_PART_ID.clone(), |context, world| {
        validate_parsed_value_has_component::<FluidContainer>(context, "pour anything into", world)
    })
    .with_if_missing("what")
    .with_placeholder_for_format_string("container")
});
static AMOUNT_PART: LazyLock<CommandFormatPart> = LazyLock::new(|| {
    any_text_part_with_validator(AMOUNT_PART_ID.clone(), validate_amount)
        .with_if_missing("how much")
        .with_placeholder_for_format_string("amount")
});

static FILL_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("fill"))
        .then(literal_part(" ").always_include_in_errors())
        .then(TARGET_PART.clone().always_include_in_errors())
        .then(literal_part(" from "))
        .then(
            SOURCE_PART
                .clone()
                .include_in_errors_if_previous_part_included(),
        )
});
static POUR_ALL_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("pour"))
        .then(
            one_of_part(nonempty![
                literal_part(" "),
                literal_part(" all "),
                literal_part(" all of ")
            ])
            .always_include_in_errors(),
        )
        .then(SOURCE_PART.clone().always_include_in_errors())
        .then(literal_part(" into "))
        .then(
            TARGET_PART
                .clone()
                .include_in_errors_if_previous_part_included(),
        )
});
static POUR_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("pour"))
        .then(literal_part(" ").always_include_in_errors())
        .then(AMOUNT_PART.clone().always_include_in_errors())
        .then(literal_part(" from "))
        .then(
            SOURCE_PART
                .clone()
                .include_in_errors_if_previous_part_included(),
        )
        .then(literal_part(" into "))
        .then(
            TARGET_PART
                .clone()
                .include_in_errors_if_previous_part_included(),
        )
});

/// Validates that a string represents an amount of fluid
fn validate_amount(context: PartValidatorContext<String>, _: &World) -> CommandPartValidateResult {
    match parse_pour_amount(&context.parsed_value) {
        Ok(_) => CommandPartValidateResult::Valid,
        Err(e) => CommandPartValidateResult::Invalid(CommandPartValidateError { details: Some(e) }),
    }
}

pub struct FillParser;
pub struct PourAllParser;
pub struct PourParser;

impl InputParser for FillParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = FILL_FORMAT.parse(input, source_entity, world)?;
        let source = parsed.get(&SOURCE_PART_ID);
        let target = parsed.get(&TARGET_PART_ID);
        build_action(source, target, PourAmount::All, world)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![FILL_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<FluidContainer>(
            entity,
            world,
            &[
                FILL_FORMAT.get_format_description().with_targeted_entity(
                    SOURCE_PART_ID.clone(),
                    entity,
                    world,
                ),
                FILL_FORMAT.get_format_description().with_targeted_entity(
                    TARGET_PART_ID.clone(),
                    entity,
                    world,
                ),
            ],
        )
    }
}

impl InputParser for PourAllParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = POUR_ALL_FORMAT.parse(input, source_entity, world)?;
        let source = parsed.get(&SOURCE_PART_ID);
        let target = parsed.get(&TARGET_PART_ID);
        build_action(source, target, PourAmount::All, world)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![POUR_ALL_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<FluidContainer>(
            entity,
            world,
            &[
                POUR_ALL_FORMAT
                    .get_format_description()
                    .with_targeted_entity(SOURCE_PART_ID.clone(), entity, world),
                POUR_ALL_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world),
            ],
        )
    }
}

impl InputParser for PourParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = POUR_FORMAT.parse(input, source_entity, world)?;
        let source = parsed.get(&SOURCE_PART_ID);
        let target = parsed.get(&TARGET_PART_ID);
        let amount = parse_pour_amount(&parsed.get(&AMOUNT_PART_ID))?;
        build_action(source, target, amount, world)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![POUR_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<FluidContainer>(
            entity,
            world,
            &[
                POUR_FORMAT.get_format_description().with_targeted_entity(
                    SOURCE_PART_ID.clone(),
                    entity,
                    world,
                ),
                POUR_FORMAT.get_format_description().with_targeted_entity(
                    TARGET_PART_ID.clone(),
                    entity,
                    world,
                ),
            ],
        )
    }
}

/// Confirms the source and target aren't the same and then builds a `PourAction` from them.
fn build_action(
    source: Entity,
    target: Entity,
    amount: PourAmount,
    world: &World,
) -> Result<Box<dyn Action>, CommandParseError> {
    if source == target {
        let target_name = Description::get_reference_name(target, Some(target), world);
        return Err(CommandParseError::Other(format!(
            "You can't pour {target_name} into itself."
        )));
    }
    Ok(Box::new(PourAction {
        source,
        target,
        amount,
        notification_sender: ActionNotificationSender::new(),
    }))
}

/* TODO
fn parse_targets(
    input: &str,
) -> Result<(String, CommandTarget, CommandTarget, PourAmount), CommandParseError> {
    if let Some(captures) = FILL_PATTERN.captures(input) {
        if let Some(target_match) = captures.name(TARGET_CAPTURE) {
            if let Some(source_match) = captures.name(SOURCE_CAPTURE) {
                let source = CommandTarget::parse(source_match.as_str());
                let target = CommandTarget::parse(target_match.as_str());
                return Ok((FILL_VERB_NAME.to_string(), source, target, PourAmount::All));
            }
        }

        return Err(InputParseError::CommandParseError {
            verb: FILL_VERB_NAME.to_string(),
            error: CommandParseError::MissingTarget,
        });
    }

    if let Some(captures) = POUR_PATTERN.captures(input) {
        if let Some(amount) = captures.name(AMOUNT_CAPTURE) {
            let amount = parse_pour_amount(amount.as_str())?;
            if let Some(target_match) = captures.name(TARGET_CAPTURE) {
                if let Some(source_match) = captures.name(SOURCE_CAPTURE) {
                    let source = CommandTarget::parse(source_match.as_str());
                    let target = CommandTarget::parse(target_match.as_str());
                    return Ok((POUR_VERB_NAME.to_string(), source, target, amount));
                }
            }
        }

        return Err(InputParseError::CommandParseError {
            verb: POUR_VERB_NAME.to_string(),
            error: CommandParseError::MissingTarget,
        });
    }

    if let Some(captures) = POUR_ALL_PATTERN.captures(input) {
        if let Some(target_match) = captures.name(TARGET_CAPTURE) {
            if let Some(source_match) = captures.name(SOURCE_CAPTURE) {
                let source = CommandTarget::parse(source_match.as_str());
                let target = CommandTarget::parse(target_match.as_str());
                return Ok((POUR_VERB_NAME.to_string(), source, target, PourAmount::All));
            }
        }

        return Err(InputParseError::CommandParseError {
            verb: POUR_VERB_NAME.to_string(),
            error: CommandParseError::MissingTarget,
        });
    }

    Err(InputParseError::UnknownCommand)
}
    */

fn parse_pour_amount(input: &str) -> Result<PourAmount, String> {
    if ALL_PATTERN.is_match(input) {
        return Ok(PourAmount::All);
    }

    let captures = AMOUNT_WITH_LITERS_PATTERN
        .captures(input)
        .or_else(|| AMOUNT_PATTERN.captures(input));

    if let Some(captures) = captures {
        if let Some(amount_match) = captures.name(AMOUNT_CAPTURE) {
            debug!("parsing amount '{}'", amount_match.as_str());
            match amount_match.as_str().parse::<f32>() {
                Ok(a) => return Ok(PourAmount::Some(Volume(a))),
                Err(_) => {
                    return Err(format!(
                        "'{}' is an invalid amount to pour.",
                        amount_match.as_str()
                    ))
                }
            }
        }
    }

    Err("You can only pour 'all' or some amount of liters.".to_string())
}

/// Makes an entity pour some liquid from one fluid container to another.
#[derive(Debug)]
pub struct PourAction {
    pub source: Entity,
    pub target: Entity,
    pub amount: PourAmount,
    notification_sender: ActionNotificationSender<Self>,
}

/// The amount of a fluid to pour.
#[derive(Debug)]
pub enum PourAmount {
    /// All of the fluid should be poured, or however much can fit in the destination container, whichever is less.
    All,
    /// A specific amount of the fluid should be poured.
    Some(Volume),
}

impl Action for PourAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let amount_in_source = world
            .get::<FluidContainer>(self.source)
            .map(|c| c.contents.get_total_volume())
            .unwrap_or(Volume(0.0));

        let target_container = world.get::<FluidContainer>(self.target);
        let amount_in_target = target_container
            .map(|c| c.contents.get_total_volume())
            .unwrap_or(Volume(0.0));
        let space_in_target = target_container
            .and_then(|c| c.volume)
            .map(|v| v - amount_in_target);

        let amount_to_pour = match self.amount {
            PourAmount::All => {
                if let Some(space_in_target) = space_in_target {
                    Volume(amount_in_source.0.min(space_in_target.0))
                } else {
                    amount_in_source
                }
            }
            PourAmount::Some(amount) => {
                if let Some(space_in_target) = space_in_target {
                    Volume(amount.0.min(amount_in_source.0).min(space_in_target.0))
                } else {
                    Volume(amount.0.min(amount_in_source.0))
                }
            }
        };

        let removed_fluids = remove_fluid(self.source, amount_to_pour, world);

        let actual_poured_amount = removed_fluids.values().copied().sum::<Volume>();
        let source_name =
            Description::get_reference_name(self.source, Some(performing_entity), world);
        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);
        if actual_poured_amount <= Volume(0.0) {
            let message = format!("You can't pour anything from {source_name} into {target_name}.");
            return ActionResult::builder()
                .with_error(performing_entity, message)
                .build_complete_no_tick(false);
        }

        if let Some(mut target_container) = world.get_mut::<FluidContainer>(self.target) {
            target_container.contents.increase(&removed_fluids);
        }

        let fluid_name = if removed_fluids.len() == 1 {
            // unwrap is safe because of the length check
            get_fluid_name(removed_fluids.iter().next().unwrap().0, world)
        } else {
            "fluid".to_string()
        };

        let first_person_message = format!("You pour {actual_poured_amount:.2}L of {fluid_name} from {source_name} into {target_name}.");

        ActionResult::builder()
            .with_message(
                performing_entity,
                first_person_message,
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${performing_entity.Name} pours some ${fluid_name} from ${source.name} into ${target.name}.")
                        .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string("fluid_name".into(), fluid_name)
                        .with_entity("source".into(), self.source)
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop pouring.".to_string(),
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

/// Removes the provided amount of fluid from the provided entity, if it contains any.
fn remove_fluid(entity: Entity, amount: Volume, world: &mut World) -> HashMap<FluidType, Volume> {
    if let Some(mut container) = world.get_mut::<FluidContainer>(entity) {
        return container.contents.reduce(amount);
    }

    HashMap::new()
}
