use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    component::{AfterActionNotification, Container, Location},
    input_parser::{InputParseError, InputParser},
    move_entity,
    notification::VerifyResult,
    BeforeActionNotification, Direction, GameMessage, MessageDelay, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const MOVE_FORMAT: &str = "go <>";
const MOVE_DIRECTION_CAPTURE: &str = "direction";

lazy_static! {
    static ref MOVE_PATTERN: Regex =
        Regex::new("^((go|move) (to (the )?)?)?(?P<direction>.*)").unwrap();
}

pub struct MoveParser;

impl InputParser for MoveParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = MOVE_PATTERN.captures(input) {
            if let Some(dir_match) = captures.name(MOVE_DIRECTION_CAPTURE) {
                if let Some(direction) = Direction::parse(dir_match.as_str()) {
                    return Ok(Box::new(MoveAction {
                        direction,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![MOVE_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct MoveAction {
    pub direction: Direction,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for MoveAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let current_location_id = world
            .get::<Location>(performing_entity)
            .expect("Moving entity should have a location")
            .id;

        let current_location = world
            .get::<Container>(current_location_id)
            .expect("Moving entity's location should be a container");
        let mut messages = HashMap::new();
        let mut should_tick = false;
        let mut was_successful = false;
        let can_receive_messages = can_receive_messages(world, performing_entity);

        if let Some((_, connection)) =
            current_location.get_connection_in_direction(&self.direction, world)
        {
            let new_room_id = connection.destination;
            move_entity(performing_entity, new_room_id, world);
            should_tick = true;
            was_successful = true;

            if can_receive_messages {
                messages.insert(
                    performing_entity,
                    vec![GameMessage::Message(
                        format!("You walk {}.", self.direction),
                        MessageDelay::Long,
                    )],
                );
            }
        } else if can_receive_messages {
            messages.insert(
                performing_entity,
                vec![GameMessage::Error(
                    "You can't move in that direction.".to_string(),
                )],
            );
        }

        ActionResult {
            messages,
            should_tick,
            is_complete: true,
            was_successful,
            post_effects: Vec::new(),
        }
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop moving.".to_string(),
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
