use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::sync::{Arc, RwLock};

use crate::{action, Action, Direction, EntityId, GameMessage, World};

lazy_static! {
    static ref MOVE_PATTERN: Regex = Regex::new("^go (.*)").unwrap();
}

/// Handles a command from a player in the provided world
pub fn handle_command(world: &Arc<RwLock<World>>, command: String, entity_id: EntityId) {
    let read_world = world.read().unwrap();
    if let Some(action) = parse_input(&command, entity_id, &read_world) {
        debug!("Parsed command into action: {action:?}");
        drop(read_world);
        world.write().unwrap().perform_action(entity_id, action);
    } else {
        read_world.send_message(
            entity_id,
            GameMessage::Error("I don't understand that.".to_string()),
        );
    }
}

/// Parses the provided string to an `Action`. Returns `None` if the string doesn't map to any action.
fn parse_input(input: &str, entity_id: EntityId, world: &World) -> Option<Box<dyn Action>> {
    if let Some(captures) = MOVE_PATTERN.captures(input) {
        if let Some(dir_match) = captures.get(1) {
            if let Some(direction) = parse_direction(dir_match.as_str()) {
                let action = action::Move { direction };
                return Some(Box::new(action));
            }
        }
    }

    if input == "look" {
        let action = action::Look;
        return Some(Box::new(action));
    }

    //TODO check entities in the presence of the entity

    None
}

/// Parses the provided string to a `Direction`. Returns `None` if the string doesn't map to any direction.
fn parse_direction(input: &str) -> Option<Direction> {
    match input {
        "n" | "north" => Some(Direction::North),
        "ne" | "northeast" => Some(Direction::NorthEast),
        "e" | "east" => Some(Direction::East),
        "se" | "southeast" => Some(Direction::SouthEast),
        "s" | "south" => Some(Direction::South),
        "sw" | "southwest" => Some(Direction::SouthWest),
        "w" | "west" => Some(Direction::West),
        "nw" | "northwest" => Some(Direction::NorthWest),
        _ => None,
    }
}
