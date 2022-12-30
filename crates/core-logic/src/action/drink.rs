use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, FluidContainer, HydrationFactor, Volume},
    despawn_entity, get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    value_change::{ValueChange, ValueChangeOperation},
    BeforeActionNotification, MessageDelay, ValueType, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// The amount of hydration gain per liter of water drank.
const HYDRATION_GAIN_PER_LITER_OF_WATER: f32 = 50.0;

/// The amount of liquid to consume in one drink.
const LITERS_PER_DRINK: Volume = Volume(0.25);

const DRINK_VERB_NAME: &str = "drink";
const DRINK_FORMAT: &str = "drink <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref DRINK_PATTERN: Regex = Regex::new("^drink (from )?(the )?(?P<name>.*)").unwrap();
}

pub struct DrinkParser;

impl InputParser for DrinkParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = DRINK_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if let Some(container) = world.get::<FluidContainer>(target_entity) {
                        if container.get_used_volume(world).0 > 0.0 {
                            // target exists and contains fluid
                            return Ok(Box::new(DrinkAction {
                                target: target_entity,
                                notification_sender: ActionNotificationSender::new(),
                            }));
                        } else {
                            // target is empty
                            let target_name =
                                get_reference_name(target_entity, source_entity, world);
                            return Err(InputParseError::CommandParseError {
                                verb: DRINK_VERB_NAME.to_string(),
                                error: CommandParseError::Other(format!("{target_name} is empty.")),
                            });
                        }
                    } else {
                        // target isn't a fluid container
                        let target_name = get_reference_name(target_entity, source_entity, world);
                        return Err(InputParseError::CommandParseError {
                            verb: DRINK_VERB_NAME.to_string(),
                            error: CommandParseError::Other(format!(
                                "You can't drink from {target_name}."
                            )),
                        });
                    }
                } else {
                    // target doesn't exist
                    return Err(InputParseError::CommandParseError {
                        verb: DRINK_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![DRINK_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<FluidContainer>(entity, world, &[DRINK_FORMAT])
    }
}

#[derive(Debug)]
pub struct DrinkAction {
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for DrinkAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target_name = get_reference_name(self.target, performing_entity, world);
        let container = match world.get::<FluidContainer>(self.target) {
            Some(s) => s,
            None => {
                return ActionResult::error(
                    performing_entity,
                    format!("You can't drink from {target_name}."),
                );
            }
        };

        let used_volume = container.get_used_volume(world);
        if used_volume.0 <= 0.0 {
            return ActionResult::error(performing_entity, format!("{target_name} is empty."));
        }

        let total_amount_to_drink = Volume(used_volume.0.min(LITERS_PER_DRINK.0));
        let fluids_to_volumes = container.get_contents_by_volume(world);

        let fluids_to_volume_to_drink = fluids_to_volumes
            .iter()
            .map(|(entity, amount)| {
                let to_drink =
                    Volume((total_amount_to_drink.0 * amount.fraction).min(amount.volume.0));
                (*entity, to_drink)
            })
            .collect::<HashMap<Entity, Volume>>();

        let hydration_increase = fluids_to_volume_to_drink
            .iter()
            .map(|(entity, volume)| {
                let hydration_factor = world
                    .get::<HydrationFactor>(*entity)
                    .unwrap_or(&HydrationFactor(0.0));

                volume.0 * HYDRATION_GAIN_PER_LITER_OF_WATER * hydration_factor.0
            })
            .sum::<f32>();

        ValueChange {
            entity: performing_entity,
            value_type: ValueType::Hydration,
            operation: ValueChangeOperation::Add,
            amount: hydration_increase,
            message: Some(format!("You take a drink from {target_name}.")),
        }
        .apply(world);

        for (entity, to_drink) in fluids_to_volume_to_drink {
            if let Some(mut volume) = world.get_mut::<Volume>(entity) {
                *volume -= to_drink;
                //TODO also adjust weight
                if volume.0 <= 0.0 {
                    despawn_entity(entity, world);
                }
            }
        }

        ActionResult::builder().build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop drinking.".to_string(),
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
