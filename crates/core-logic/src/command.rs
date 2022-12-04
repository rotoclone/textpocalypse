use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::{
    action::{self},
    command_parser::{parse_command, CommandError, InputParseError},
    component::{CustomCommandParser, Room},
    perform_action, send_message, Action, Direction, GameMessage, Location, StandardCommandParsers,
    World,
};

/// Handles input from a player.
pub fn handle_input(world: &Arc<RwLock<World>>, input: String, entity: Entity) {
    let read_world = world.read().unwrap();
    if let Ok(action) = parse_input(&input, entity, &read_world) {
        debug!("Parsed command into action: {action:?}");
        drop(read_world);
        perform_action(&mut world.write().unwrap(), entity, action);
    } else {
        //TODO handle errors better
        send_message(
            &read_world,
            entity,
            GameMessage::Error("I don't understand that.".to_string()),
        );
    }
}

//TODO this is a bad name for this
enum GeneralInputParseError {
    InputToCommand(InputParseError),
    CommandToAction(CommandError),
}

impl From<InputParseError> for GeneralInputParseError {
    fn from(e: InputParseError) -> Self {
        GeneralInputParseError::InputToCommand(e)
    }
}

impl From<CommandError> for GeneralInputParseError {
    fn from(e: CommandError) -> Self {
        GeneralInputParseError::CommandToAction(e)
    }
}

/// Parses the provided string to an `Action`. Returns `None` if the string doesn't map to any action.
fn parse_input(
    input: &str,
    entity: Entity,
    world: &World,
) -> Result<Box<dyn Action>, GeneralInputParseError> {
    let mut custom_parsers = Vec::new();
    for found_entity in find_entities_in_presence_of(entity, world) {
        if let Some(command_parser) = world.get::<CustomCommandParser>(found_entity) {
            debug!("Found custom command parser on {found_entity:?}");
            //TODO prevent duplicate parsers from being registered
            custom_parsers.extend(&command_parser.parsers);
        }
    }

    let parsers = world
        .resource::<StandardCommandParsers>()
        .parsers
        .iter()
        .chain(custom_parsers);

    let command = parse_command(input, parsers)?;
    let action = command.to_action(entity, world)?;

    Ok(action)
}

/// Finds all the entities the provided entity can currently directly interact with.
fn find_entities_in_presence_of(entity: Entity, world: &World) -> HashSet<Entity> {
    let location_id = world
        .get::<Location>(entity)
        .expect("Entity should have a location")
        .id;

    // TODO also include entities in the provided entity's inventory
    // TODO handle entities not located in a room
    let room = world
        .get::<Room>(location_id)
        .expect("Entity's location should be a room");

    room.entities.clone()
}
