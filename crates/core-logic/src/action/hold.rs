use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{
        ActionEndNotification, AfterActionPerformNotification, HeldItems, HoldError, Item,
        Location, UnholdError,
    },
    find_wearing_entity, get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::{Notification, VerifyResult},
    BeforeActionNotification, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ThirdPersonMessage,
    ThirdPersonMessageLocation,
};

const HOLD_VERB_NAME: &str = "hold";
const UNHOLD_VERB_NAME: &str = "put away";
const HOLD_FORMAT: &str = "hold <>";
const UNHOLD_FORMAT: &str = "put away <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref HOLD_PATTERN: Regex =
        Regex::new("^(hold|equip|wield|unholster|take out) (the )?(?P<name>.*)").unwrap();
    static ref UNHOLD_PATTERN: Regex =
        Regex::new("^(unhold|unequip|unwield|holster|stow|put away) (the )?(?P<name>.*)").unwrap();
}

pub struct HoldParser;

impl InputParser for HoldParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (captures, verb_name, should_be_held) =
            if let Some(captures) = HOLD_PATTERN.captures(input) {
                (captures, HOLD_VERB_NAME, true)
            } else if let Some(captures) = UNHOLD_PATTERN.captures(input) {
                (captures, UNHOLD_VERB_NAME, false)
            } else {
                return Err(InputParseError::UnknownCommand);
            };

        if let Some(target_match) = captures.name(NAME_CAPTURE) {
            let target = CommandTarget::parse(target_match.as_str());
            if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                if world.get::<Item>(target_entity).is_some() {
                    // target exists and is holdable
                    return Ok(Box::new(HoldAction {
                        target: target_entity,
                        should_be_held,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                } else {
                    // target isn't holdable
                    let target_name = get_reference_name(target_entity, Some(source_entity), world);
                    return Err(InputParseError::CommandParseError {
                        verb: HOLD_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!(
                            "You can't {verb_name} {target_name}."
                        )),
                    });
                }
            } else {
                // target doesn't exist
                return Err(InputParseError::CommandParseError {
                    verb: verb_name.to_string(),
                    error: CommandParseError::TargetNotFound(target),
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![HOLD_FORMAT.to_string(), UNHOLD_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<Item>(entity, world, &[HOLD_FORMAT, UNHOLD_FORMAT])
    }
}

#[derive(Debug)]
pub struct HoldAction {
    pub target: Entity,
    pub should_be_held: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for HoldAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = get_reference_name(target, Some(performing_entity), world);

        if self.should_be_held {
            match HeldItems::hold(performing_entity, target, world) {
                Ok(()) => (),
                Err(HoldError::CannotHold) => {
                    return ActionResult::builder()
                        .with_error(performing_entity, "You can't hold things.".to_string())
                        .build_complete_no_tick(false)
                }
                Err(HoldError::CannotBeHeld) => {
                    return ActionResult::builder()
                        .with_error(performing_entity, format!("You can't hold {target_name}."))
                        .build_complete_no_tick(false)
                }
                Err(HoldError::AlreadyHeld) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You're already holding {target_name}."),
                        )
                        .build_complete_no_tick(false)
                }
                Err(HoldError::NotEnoughHands) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You don't have enough free hands to hold {target_name}."),
                        )
                        .build_complete_no_tick(false);
                }
            }
        } else {
            match HeldItems::unhold(performing_entity, target, world) {
                Ok(()) => (),
                Err(UnholdError::NotHolding) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You're not holding {target_name}."),
                        )
                        .build_complete_no_tick(false)
                }
            }
        }

        let (take_out_or_put_away, takes_out_or_puts_away) = if self.should_be_held {
            ("take out", "takes out")
        } else {
            ("put away", "puts away")
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {take_out_or_put_away} {target_name}."),
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
                .add_entity_name(performing_entity)
                .add_string(format!(" {takes_out_or_puts_away} "))
                .add_entity_name(target)
                .add_string(".".to_string()),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        if self.should_be_held {
            ActionInterruptResult::message(
                performing_entity,
                "You stop holding things.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::None,
            )
        } else {
            ActionInterruptResult::message(
                performing_entity,
                "You stop putting things away.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::None,
            )
        }
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

/// Verifies that the entity trying to hold an item contains it.
pub fn verify_has_item_to_hold(
    notification: &Notification<VerifyActionNotification, HoldAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(location) = world.get::<Location>(item) {
        if location.id == performing_entity {
            return VerifyResult::valid();
        }
    }

    let item_name = get_reference_name(item, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You don't have {item_name}.")),
    )
}

/// Verifies that the entity trying to hold an item is not wearing it.
pub fn verify_not_wearing_item_to_hold(
    notification: &Notification<VerifyActionNotification, HoldAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(wearing_entity) = find_wearing_entity(item, world) {
        if wearing_entity == performing_entity {
            let item_name = get_reference_name(item, Some(performing_entity), world);
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!(
                    "You'll have to take off {item_name} before you can hold it."
                )),
            );
        }
    }

    VerifyResult::valid()
}
