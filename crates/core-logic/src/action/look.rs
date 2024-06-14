use std::collections::HashSet;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    component::{
        ActionEndNotification, AfterActionPerformNotification, Connection, Container, Description,
        Room,
    },
    game_map::Coordinates,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, DetailedEntityDescription, EntityDescription, GameMessage,
    InternalMessageCategory, MessageCategory, MessageDelay, RoomDescription,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const LOOK_VERB_NAME: &str = "look";
const DETAILED_LOOK_VERB_NAME: &str = "examine";
const LOOK_FORMAT: &str = "look <>";
const DETAILED_LOOK_FORMAT: &str = "examine <>";
const LOOK_TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref LOOK_PATTERN: Regex =
        Regex::new("^(l|look)($|( (at )?(the )?(?P<target>.*)))").unwrap();
    static ref DETAILED_LOOK_PATTERN: Regex =
        Regex::new("^(x|ex|examine)($|( (the )?(?P<target>.*)))").unwrap();
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

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        input_formats_if_has_component::<Description>(
            entity,
            world,
            &[LOOK_FORMAT, DETAILED_LOOK_FORMAT],
        )
    }
}

/// Shows an entity the description of something.
#[derive(Debug)]
pub struct LookAction {
    pub target: Entity,
    pub detailed: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for LookAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, performing_entity) {
            return ActionResult::none();
        }

        let target = world.entity(self.target);

        if let Some(room) = target.get::<Room>() {
            if let Some(container) = target.get::<Container>() {
                if let Some(coords) = target.get::<Coordinates>() {
                    return ActionResult::builder()
                        .with_game_message(
                            performing_entity,
                            GameMessage::Room(RoomDescription::from_room(
                                room,
                                container,
                                coords,
                                performing_entity,
                                world,
                            )),
                        )
                        .build_complete_no_tick(true);
                }
            }
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
                GameMessage::Entity(EntityDescription::for_entity(
                    performing_entity,
                    self.target,
                    desc,
                    world,
                ))
            };
            return ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true);
        } else if let Some(connection) = target.get::<Connection>() {
            // the target is a connection without a description, which means it must be just an open connection, so let the performing entity look
            // through it
            let room = world
                .get::<Room>(connection.destination)
                .expect("connection destination should be a room");
            let container = world
                .get::<Container>(connection.destination)
                .expect("connection destination should be a container");
            let coordinates = world
                .get::<Coordinates>(connection.destination)
                .expect("connection destination should have coordinates");
            let room_description_message = GameMessage::Room(RoomDescription::from_room(
                room,
                container,
                coordinates,
                performing_entity,
                world,
            ));
            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    format!("To the {}, you see:", connection.direction),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::None,
                )
                .with_game_message(performing_entity, room_description_message)
                .build_complete_no_tick(true);
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }

    fn interrupt(&self, _: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::none()
    }

    fn may_require_tick(&self) -> bool {
        false
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
