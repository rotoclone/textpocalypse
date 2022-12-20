use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    component::{Description, Room},
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    BeforeActionNotification, DetailedEntityDescription, EntityDescription, GameMessage,
    RoomDescription, VerifyActionNotification, World,
};

use super::{Action, ActionNotificationSender, ActionResult};

const LOOK_VERB_NAME: &str = "look";
const DETAILED_LOOK_VERB_NAME: &str = "examine";
const LOOK_FORMAT: &str = "look <>";
const DETAILED_LOOK_FORMAT: &str = "examine <>";
const LOOK_TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref LOOK_PATTERN: Regex = Regex::new("^l(ook)?( (at )?(the )?(?P<target>.*))?").unwrap();
    static ref DETAILED_LOOK_PATTERN: Regex =
        Regex::new("^(x|ex(amine)?)( (the )?(?P<target>.*))?").unwrap();
}

pub struct LookParser;

impl InputParser for LookParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (captures, verb_name, detailed) = if let Some(captures) = LOOK_PATTERN.captures(input) {
            (captures, LOOK_VERB_NAME, false)
        } else if let Some(captures) = DETAILED_LOOK_PATTERN.captures(input) {
            (captures, DETAILED_LOOK_VERB_NAME, true)
        } else {
            return Err(InputParseError::UnknownCommand);
        };

        if let Some(target_match) = captures.name(LOOK_TARGET_CAPTURE) {
            // looking at something specific
            let target = CommandTarget::parse(target_match.as_str());
            if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                // looking at something they can see
                return Ok(Box::new(LookAction {
                    target: target_entity,
                    detailed,
                    notification_sender: ActionNotificationSender::new(),
                }));
            } else {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name.to_string(),
                    error: CommandParseError::TargetNotFound(target),
                });
            }
        } else {
            // just looking in general
            if let Some(target) = CommandTarget::Here.find_target_entity(source_entity, world) {
                return Ok(Box::new(LookAction {
                    target,
                    detailed,
                    notification_sender: ActionNotificationSender::new(),
                }));
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![LOOK_FORMAT.to_string(), DETAILED_LOOK_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<Description>(
            entity,
            world,
            &[LOOK_FORMAT, DETAILED_LOOK_FORMAT],
        )
    }
}

#[derive(Debug)]
struct LookAction {
    target: Entity,
    detailed: bool,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for LookAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, performing_entity) {
            return ActionResult::none();
        }

        let target = world.entity(self.target);

        if let Some(room) = target.get::<Room>() {
            return ActionResult::builder()
                .with_game_message(
                    performing_entity,
                    GameMessage::Room(RoomDescription::from_room(room, performing_entity, world)),
                )
                .build_complete_no_tick();
        }

        if let Some(desc) = target.get::<Description>() {
            let message = if self.detailed {
                GameMessage::DetailedEntity(DetailedEntityDescription::for_entity(
                    performing_entity,
                    self.target,
                    desc,
                    world,
                ))
            } else {
                GameMessage::Entity(EntityDescription::for_entity(self.target, desc, world))
            };
            return ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick();
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }

    fn may_require_tick(&self) -> bool {
        false
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
}
