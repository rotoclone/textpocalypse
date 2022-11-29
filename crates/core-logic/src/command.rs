use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::sync::{Arc, RwLock};

use crate::{
    action::{self},
    perform_action,
    room::Room,
    send_message, Action, Direction, GameMessage, Location, World,
};

const MOVE_DIRECTION_CAPTURE: &str = "direction";
const LOOK_TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref MOVE_PATTERN: Regex =
        Regex::new("^((go|move) (to (the )?)?)?(?P<direction>.*)").unwrap();
    static ref LOOK_PATTERN: Regex = Regex::new("^l(ook)?( (at )?(the )?(?P<target>.*))?").unwrap();
    static ref SELF_TARGET_PATTERN: Regex = Regex::new("^(me|myself|self)$").unwrap();
    static ref HERE_TARGET_PATTERN: Regex = Regex::new("^(here)$").unwrap();
}

/// Handles a command from a player in the provided world
pub fn handle_command(world: &Arc<RwLock<World>>, command: String, entity_id: Entity) {
    let read_world = world.read().unwrap();
    if let Some(action) = parse_input(&command, entity_id, &read_world) {
        debug!("Parsed command into action: {action:?}");
        drop(read_world);
        perform_action(&mut world.write().unwrap(), entity_id, action);
    } else {
        send_message(
            &read_world,
            entity_id,
            GameMessage::Error("I don't understand that.".to_string()),
        );
    }
}

/// Parses the provided string to an `Action`. Returns `None` if the string doesn't map to any action.
fn parse_input(input: &str, entity_id: Entity, world: &World) -> Option<Box<dyn Action>> {
    if let Some(captures) = MOVE_PATTERN.captures(input) {
        if let Some(dir_match) = captures.name(MOVE_DIRECTION_CAPTURE) {
            if let Some(direction) = parse_direction(dir_match.as_str()) {
                let action = action::Move { direction };
                return Some(Box::new(action));
            }
        }
    }

    if let Some(captures) = LOOK_PATTERN.captures(input) {
        if let Some(target_match) = captures.name(LOOK_TARGET_CAPTURE) {
            if let Some(target) = find_entity_by_name(target_match.as_str(), entity_id, world) {
                let action = action::Look { target };
                return Some(Box::new(action));
            }
        } else {
            let action = action::Look {
                target: world
                    .get::<Location>(entity_id)
                    .expect("Looking entity should have a location")
                    .id,
            };
            return Some(Box::new(action));
        }
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

/// Finds an entity from the perspective of another entity, if it exists.
fn find_entity_by_name(
    entity_name: &str,
    looking_entity_id: Entity,
    world: &World,
) -> Option<Entity> {
    let entity_name = entity_name.to_lowercase();
    debug!("Finding {entity_name:?} from the perspective of {looking_entity_id:?}");

    if SELF_TARGET_PATTERN.is_match(&entity_name) {
        return Some(looking_entity_id);
    }

    let room_id = world
        .get::<Location>(looking_entity_id)
        .expect("Looking entity should have a location")
        .id;
    if HERE_TARGET_PATTERN.is_match(&entity_name) {
        return Some(room_id);
    }

    //TODO also search the looking entity's inventory
    let room = world
        .get::<Room>(room_id)
        .expect("Looking entity's location should be a room");
    room.find_entity_by_name(&entity_name, world)
}
