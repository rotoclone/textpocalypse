use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Fluid, FluidContainer, FluidType, Volume},
    get_fluid_name, get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    BeforeActionNotification, MessageDelay, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const FILL_VERB_NAME: &str = "fill";
const POUR_VERB_NAME: &str = "pour";

const FILL_FORMAT: &str = "fill <> from <>";
const POUR_FORMAT: &str = "pour <> from <> into <>";

const SOURCE_CAPTURE: &str = "source";
const TARGET_CAPTURE: &str = "target";
const AMOUNT_CAPTURE: &str = "amount";

lazy_static! {
    static ref FILL_PATTERN: Regex =
        Regex::new("^fill (the )?(?P<target>.*) from (the )?(?P<source>.*)").unwrap();
    static ref POUR_PATTERN: Regex =
        Regex::new("^pour (?P<amount>.*) from (the )?(?P<source>.*) into (the )?(?P<target>.*)")
            .unwrap();
    static ref POUR_ALL_PATTERN: Regex =
        Regex::new("^pour( all( of)?)? (the )?(?P<source>.*) into (the )?(?P<target>.*)").unwrap();
    static ref ALL_PATTERN: Regex = Regex::new("^all$").unwrap();
    static ref AMOUNT_WITH_LITERS_PATTERN: Regex =
        Regex::new("^(?P<amount>[^ ]*)(L| L| liter| liters)$").unwrap();
    static ref AMOUNT_PATTERN: Regex = Regex::new("^(?P<amount>.*)").unwrap();
}

pub struct PourParser;

impl InputParser for PourParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (verb_name, source_target, target_target, amount) = parse_targets(input)?;

        let source = match source_target.find_target_entity(entity, world) {
            Some(e) => e,
            None => {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::TargetNotFound(source_target),
                })
            }
        };

        if world.get::<FluidContainer>(source).is_none() {
            let source_name = get_reference_name(source, entity, world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(format!("{source_name} is not a fluid container.")),
            });
        }

        let target = match target_target.find_target_entity(entity, world) {
            Some(e) => e,
            None => {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::TargetNotFound(target_target),
                })
            }
        };

        if source == target {
            let target_name = get_reference_name(target, entity, world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(format!(
                    "You can't pour {target_name} into itself."
                )),
            });
        }

        if world.get::<FluidContainer>(target).is_none() {
            let target_name = get_reference_name(target, entity, world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(format!("{target_name} is not a fluid container.")),
            });
        }

        Ok(Box::new(PourAction {
            source,
            target,
            amount,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![FILL_FORMAT.to_string(), POUR_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<FluidContainer>(entity, world, &[FILL_FORMAT, POUR_FORMAT])
    }
}

fn parse_targets(
    input: &str,
) -> Result<(String, CommandTarget, CommandTarget, PourAmount), InputParseError> {
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

fn parse_pour_amount(input: &str) -> Result<PourAmount, InputParseError> {
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
                    return Err(InputParseError::CommandParseError {
                        verb: POUR_VERB_NAME.to_string(),
                        error: CommandParseError::Other(
                            "That is an invalid amount to pour.".to_string(),
                        ),
                    })
                }
            }
        }
    }

    Err(InputParseError::CommandParseError {
        verb: POUR_VERB_NAME.to_string(),
        error: CommandParseError::Other(
            "You can only pour 'all' or some amount of liters.".to_string(),
        ),
    })
}

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
            .and_then(|c| c.contents.as_ref())
            .map(|f| f.get_total_volume())
            .unwrap_or(Volume(0.0));

        let target_container = world.get::<FluidContainer>(self.target);
        let amount_in_target = target_container
            .and_then(|c| c.contents.as_ref())
            .map(|f| f.get_total_volume())
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
        let source_name = get_reference_name(self.source, performing_entity, world);
        let target_name = get_reference_name(self.target, performing_entity, world);
        if actual_poured_amount <= Volume(0.0) {
            let message = format!("You can't pour anything from {source_name} into {target_name}.");
            return ActionResult::builder()
                .with_error(performing_entity, message)
                .build_complete_no_tick(false);
        }

        if let Some(mut target_container) = world.get_mut::<FluidContainer>(self.target) {
            let fluid = target_container.contents.get_or_insert_with(|| Fluid {
                contents: HashMap::new(),
            });

            fluid.increase(&removed_fluids);
        }

        let fluid_name = if removed_fluids.len() == 1 {
            // unwrap is safe because of the length check
            get_fluid_name(removed_fluids.iter().next().unwrap().0, world)
        } else {
            "fluid".to_string()
        };

        let message = format!("You pour {actual_poured_amount:.2}L of {fluid_name} from {source_name} into {target_name}.");

        ActionResult::builder()
            .with_message(performing_entity, message, MessageDelay::Short)
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop pouring.".to_string(),
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

    fn send_after_notification(
        &self,
        notification_type: AfterActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_notification(notification_type, self, world);
    }
}

/// Removes the provided amount of fluid from the provided entity, if it contains any.
fn remove_fluid(entity: Entity, amount: Volume, world: &mut World) -> HashMap<FluidType, Volume> {
    if let Some(mut container) = world.get_mut::<FluidContainer>(entity) {
        if let Some(fluid) = &mut container.contents {
            return fluid.reduce(amount);
        }
    }

    HashMap::new()
}
