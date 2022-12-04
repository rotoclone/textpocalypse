use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    component::{Description, Location, OpenState, Room},
    input_parser::{InputParseError, InputParser},
    move_entity, Direction, GameMessage, RoomDescription,
};

use super::{Action, ActionResult};

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
                    return Ok(Box::new(MoveAction { direction }));
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }
}

#[derive(Debug)]
struct MoveAction {
    direction: Direction,
}

impl Action for MoveAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let current_room_id = world
            .get::<Location>(performing_entity)
            .expect("Moving entity should have a location")
            .id;

        let current_room = world
            .get::<Room>(current_room_id)
            .expect("Moving entity's location should be a room");
        let mut messages = HashMap::new();
        let mut should_tick = false;
        let can_receive_messages = can_receive_messages(world, performing_entity);

        if let Some((connecting_entity, connection)) =
            current_room.get_connection_in_direction(&self.direction, world)
        {
            //TODO if the connecting entity is closed and can be opened, try to open it first
            if let Some(message) = invalid_move_message(connecting_entity, world) {
                messages.insert(performing_entity, vec![message]);
            } else {
                let new_room_id = connection.destination;
                move_entity(performing_entity, new_room_id, world);
                should_tick = true;

                if can_receive_messages {
                    let new_room = world
                        .get::<Room>(new_room_id)
                        .expect("Destination entity should be a room");
                    let room_desc = RoomDescription::from_room(new_room, performing_entity, world);
                    messages.insert(performing_entity, vec![GameMessage::Room(room_desc)]);
                }
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
        }
    }
}

/// Determines if the provided entity can be moved through
fn invalid_move_message(entity: Entity, world: &World) -> Option<GameMessage> {
    world
        .get::<OpenState>(entity)
        .map(|state| {
            if !state.is_open {
                let message = world
                    .get::<Description>(entity)
                    .map_or("It's closed.".to_string(), |desc| {
                        format!("The {} is closed.", desc.name)
                    });
                Some(GameMessage::Message(message))
            } else {
                None
            }
        })
        .unwrap_or(None)
}
