use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{
        ActionEndNotification, AfterActionPerformNotification, RemoveError, Wearable, WornItems,
    },
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    is_living_entity,
    notification::{Notification, VerifyResult},
    BeforeActionNotification, Description, GameMessage, InternalMessageCategory, MessageCategory,
    MessageDelay, SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ThirdPersonMessage,
    ThirdPersonMessageLocation,
};

const REMOVE_VERB_NAME: &str = "remove";
const REMOVE_FORMAT: &str = "remove <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref REMOVE_PATTERN: Regex =
        Regex::new("^(remove|take off) (the )?(?P<name>.*)").unwrap();
}

pub struct RemoveParser;

impl InputParser for RemoveParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = REMOVE_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Wearable>(target_entity).is_some() {
                        // target exists and is wearable
                        return Ok(Box::new(RemoveAction {
                            wearing_entity: source_entity,
                            target: target_entity,
                            notification_sender: ActionNotificationSender::new(),
                        }));
                    } else {
                        // target isn't wearable
                        let target_name = Description::get_reference_name(
                            target_entity,
                            Some(source_entity),
                            world,
                        );
                        return Err(InputParseError::CommandParseError {
                            verb: REMOVE_VERB_NAME.to_string(),
                            error: CommandParseError::Other(format!(
                                "You're not wearing {target_name}, and you couldn't if you tried."
                            )),
                        });
                    }
                } else {
                    // target doesn't exist
                    return Err(InputParseError::CommandParseError {
                        verb: REMOVE_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![REMOVE_FORMAT.to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        input_formats_if_has_component::<Wearable>(entity, world, &[REMOVE_FORMAT])
    }
}

/// Makes an entity remove an item it's wearing.
#[derive(Debug)]
pub struct RemoveAction {
    pub wearing_entity: Entity,
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for RemoveAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let wearing_entity = self.wearing_entity;
        let target = self.target;
        let wearing_entity_name =
            Description::get_reference_name(wearing_entity, Some(performing_entity), world);
        let target_name = Description::get_reference_name(target, Some(performing_entity), world);

        match WornItems::remove(wearing_entity, target, world) {
            Ok(()) => (),
            Err(RemoveError::NotWorn) => {
                if wearing_entity == performing_entity {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You're not wearing {target_name}."),
                        )
                        .build_complete_no_tick(false);
                }

                return ActionResult::builder()
                    .with_error(
                        performing_entity,
                        format!("{wearing_entity_name} is not wearing {target_name}."),
                    )
                    .build_complete_no_tick(false);
            }
        }

        let mut result_builder = ActionResult::builder();

        if wearing_entity == performing_entity {
            result_builder = result_builder
                .with_message(
                    performing_entity,
                    format!("You take off {target_name}."),
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
                    .add_string(" takes off ".to_string())
                    .add_name(target)
                    .add_string(".".to_string()),
                    world,
                );
        } else {
            result_builder = result_builder
                .with_message(
                    performing_entity,
                    format!("You take {target_name} off of {wearing_entity_name}."),
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
                    .add_string(" takes ".to_string())
                    .add_name(target)
                    .add_string(" off of ")
                    .add_name(wearing_entity)
                    .add_string(".".to_string()),
                    world,
                );
        }
        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult {
        if self.wearing_entity == performing_entity {
            ActionInterruptResult::message(
                performing_entity,
                "You stop taking things off.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::None,
            )
        } else {
            let wearing_entity_name = Description::get_reference_name(
                self.wearing_entity,
                Some(performing_entity),
                world,
            );
            ActionInterruptResult::message(
                performing_entity,
                format!("You stop taking things off of {wearing_entity_name}."),
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

/// Prevents removing items from living entities other than the one performing the action.
pub fn prevent_remove_from_other_living_entity(
    notification: &Notification<VerifyActionNotification, RemoveAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let wearing_entity = notification.contents.wearing_entity;

    if performing_entity != wearing_entity && is_living_entity(wearing_entity, world) {
        let wearing_entity_name =
            Description::get_reference_name(wearing_entity, Some(performing_entity), world);
        let message = format!("You can't remove anything from {wearing_entity_name}.");
        return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
    }

    VerifyResult::valid()
}
