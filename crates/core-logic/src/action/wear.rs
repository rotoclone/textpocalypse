use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{
        ActionEndNotification, AfterActionPerformNotification, Location, WearError, Wearable,
        WornItems,
    },
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const WEAR_VERB_NAME: &str = "wear";
const WEAR_FORMAT: &str = "wear <>";
const NAME_CAPTURE: &str = "name";

static WEAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(wear|put on) (the )?(?P<name>.*)").unwrap());

pub struct WearParser;

impl InputParser for WearParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = WEAR_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Wearable>(target_entity).is_some() {
                        // target exists and is wearable
                        return Ok(Box::new(WearAction {
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
                            verb: WEAR_VERB_NAME.to_string(),
                            error: CommandParseError::Other(format!(
                                "You can't wear {target_name}."
                            )),
                        });
                    }
                } else {
                    // target doesn't exist
                    return Err(InputParseError::CommandParseError {
                        verb: WEAR_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![WEAR_FORMAT.to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        input_formats_if_has_component::<Wearable>(entity, world, &[WEAR_FORMAT])
    }
}

/// Makes an entity put on a wearable item.
#[derive(Debug)]
pub struct WearAction {
    //TODO allow putting things on entities other than yourself
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for WearAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = Description::get_reference_name(target, Some(performing_entity), world);

        match WornItems::wear(performing_entity, target, world) {
            Ok(()) => (),
            Err(WearError::CannotWear) => {
                return ActionResult::builder()
                    .with_error(performing_entity, "You can't wear things.".to_string())
                    .build_complete_no_tick(false)
            }
            Err(WearError::NotWearable) => {
                return ActionResult::builder()
                    .with_error(performing_entity, format!("You can't wear {target_name}."))
                    .build_complete_no_tick(false)
            }
            Err(WearError::AlreadyWorn) => {
                return ActionResult::builder()
                    .with_error(
                        performing_entity,
                        format!("You're already wearing {target_name}."),
                    )
                    .build_complete_no_tick(false)
            }
            Err(WearError::TooThick { other_item, .. }) => {
                let other_item_name =
                    Description::get_reference_name(other_item, Some(performing_entity), world);
                return ActionResult::builder()
                    .with_error(
                        performing_entity,
                        format!("You can't fit {target_name} over {other_item_name}."),
                    )
                    .build_complete_no_tick(false);
            }
        }

        ActionResult::builder()
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${performing_entity.Name} ${performing_entity.you:put/puts} on ${target.name}.")
                        .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_entity("target".into(), target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop putting things on.".to_string(),
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

/// Verifies that the entity trying to put on an item contains it.
pub fn verify_has_item_to_wear(
    notification: &Notification<VerifyActionNotification, WearAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(location) = world.get::<Location>(item) {
        if location.id == performing_entity {
            return VerifyResult::valid();
        }
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You don't have {item_name}.")),
    )
}
