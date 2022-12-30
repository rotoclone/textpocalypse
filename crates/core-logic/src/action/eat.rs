use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Edible},
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

/// The amount of satiety gain per calorie eaten.
const SATIETY_GAIN_PER_CALORIE: f32 = 0.01;

const EAT_VERB_NAME: &str = "eat";
const EAT_FORMAT: &str = "eat <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref EAT_PATTERN: Regex = Regex::new("^eat (the )?(?P<name>.*)").unwrap();
}

pub struct EatParser;

impl InputParser for EatParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = EAT_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Edible>(target_entity).is_some() {
                        // target exists and is edible
                        return Ok(Box::new(EatAction {
                            target: target_entity,
                            notification_sender: ActionNotificationSender::new(),
                        }));
                    } else {
                        // target isn't edible
                        let target_name = get_reference_name(target_entity, world);
                        return Err(InputParseError::CommandParseError {
                            verb: EAT_VERB_NAME.to_string(),
                            error: CommandParseError::Other(format!(
                                "You can't eat {target_name}."
                            )),
                        });
                    }
                } else {
                    // target doesn't exist
                    return Err(InputParseError::CommandParseError {
                        verb: EAT_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![EAT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<Edible>(entity, world, &[EAT_FORMAT])
    }
}

#[derive(Debug)]
pub struct EatAction {
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for EatAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target_name = get_reference_name(self.target, world);
        let edible = match world.get::<Edible>(self.target) {
            Some(s) => s,
            None => {
                return ActionResult::error(
                    performing_entity,
                    format!("You can't eat {target_name}."),
                );
            }
        };

        ValueChange {
            entity: performing_entity,
            value_type: ValueType::Satiety,
            operation: ValueChangeOperation::Add,
            amount: f32::from(edible.calories) * SATIETY_GAIN_PER_CALORIE,
            message: Some(format!("You eat {target_name}.")),
        }
        .apply(world);

        despawn_entity(self.target, world);

        ActionResult::builder().build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop eating.".to_string(),
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
