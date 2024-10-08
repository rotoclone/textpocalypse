use std::{collections::HashSet, fmt::Display, sync::LazyLock};

use bevy_ecs::prelude::*;
use log::debug;
use regex::Regex;

use crate::{
    action::Action,
    component::{Container, CustomInputParser, Location},
    Direction, StandardInputParsers,
};

static SELF_TARGET_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(me|myself|self)$").unwrap());
static HERE_TARGET_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("^(here)$").unwrap());

/// Parses the provided string to an `Action`.
pub fn parse_input(
    input: &str,
    source_entity: Entity,
    world: &World,
) -> Result<Box<dyn Action>, InputParseError> {
    let parsers = find_parsers_relevant_for(source_entity, world);

    //TODO run validators for this action?
    parse_input_with(input, source_entity, world, parsers)
}

/// Finds all the parsers relevant for input from the provided entity.
pub fn find_parsers_relevant_for(
    entity: Entity,
    world: &World,
) -> impl Iterator<Item = &Box<dyn InputParser>> {
    let mut custom_parsers = Vec::new();
    for found_entity in find_entities_in_presence_of(entity, world) {
        if let Some(input_parser) = world.get::<CustomInputParser>(found_entity) {
            debug!("Found custom input parser on {found_entity:?}");
            //TODO prevent duplicate parsers from being registered
            custom_parsers.extend(&input_parser.parsers);
        }
    }

    world
        .resource::<StandardInputParsers>()
        .parsers
        .iter()
        .chain(custom_parsers)
}

/// Finds all the entities the provided entity can currently directly interact with.
fn find_entities_in_presence_of(entity: Entity, world: &World) -> HashSet<Entity> {
    let location_id = world
        .get::<Location>(entity)
        .expect("Entity should have a location")
        .id;

    // include entities in the provided entity's location
    let location = world
        .get::<Container>(location_id)
        .expect("Entity's location should be a container");

    let mut entities = location.get_entities(entity, world).clone();

    // include entities in the provided entity's inventory
    if let Some(inventory) = world.get::<Container>(entity) {
        entities.extend(inventory.get_entities(entity, world).clone());
    }

    entities
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommandTarget {
    Myself,
    Here,
    Direction(Direction),
    Named(CommandTargetName),
}

impl Display for CommandTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandTarget::Myself => "me".fmt(f),
            CommandTarget::Here => "here".fmt(f),
            CommandTarget::Direction(dir) => dir.fmt(f),
            CommandTarget::Named(name) => name.fmt(f),
        }
    }
}

impl CommandTarget {
    /// Parses the provided string to a `CommandTarget`.
    pub fn parse(input: &str) -> CommandTarget {
        if SELF_TARGET_PATTERN.is_match(input) {
            return CommandTarget::Myself;
        }

        if HERE_TARGET_PATTERN.is_match(input) {
            return CommandTarget::Here;
        }

        if let Some(dir) = Direction::parse(input) {
            return CommandTarget::Direction(dir);
        }

        CommandTarget::Named(CommandTargetName {
            name: input.to_lowercase(),
            location_chain: Vec::new(), //TODO populate this
        })
    }

    /// Finds the entity described by this target, if it exists from the perspective of the looking entity.
    pub fn find_target_entity(&self, looking_entity: Entity, world: &World) -> Option<Entity> {
        debug!("Finding {self:?} from the perspective of {looking_entity:?}");

        match self {
            CommandTarget::Myself => Some(looking_entity),
            CommandTarget::Here => {
                let location_id = world
                    .get::<Location>(looking_entity)
                    .expect("Looking entity should have a location")
                    .id;
                Some(location_id)
            }
            CommandTarget::Direction(dir) => {
                let location_id = world
                    .get::<Location>(looking_entity)
                    .expect("Looking entity should have a location")
                    .id;
                let container = world
                    .get::<Container>(location_id)
                    .expect("Looking entity's location should be a container");
                if let Some((connecting_entity, _)) =
                    container.get_connection_in_direction(dir, looking_entity, world)
                {
                    Some(connecting_entity)
                } else {
                    None
                }
            }
            CommandTarget::Named(target_name) => {
                target_name.find_target_entity(looking_entity, world)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandTargetName {
    pub name: String,
    pub location_chain: Vec<String>,
}

impl Display for CommandTargetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO include location chain
        self.name.fmt(f)
    }
}

impl CommandTargetName {
    /// Finds the entity described by this target, if it exists from the perspective of the looking entity.
    pub fn find_target_entity(&self, looking_entity: Entity, world: &World) -> Option<Entity> {
        //TODO take location chain into account

        // search the looking entity's inventory
        // TODO allow callers to define whether inventory or location should be searched first
        if let Some(container) = world.get::<Container>(looking_entity) {
            if let Some(found_entity) =
                container.find_entity_by_name(&self.name, looking_entity, world)
            {
                return Some(found_entity);
            }
        }

        // search the looking entity's location
        let location_id = world
            .get::<Location>(looking_entity)
            .expect("Looking entity should have a location")
            .id;
        let location = world
            .get::<Container>(location_id)
            .expect("Looking entity's location should be a container");
        location.find_entity_by_name(&self.name, looking_entity, world)
    }

    /// Finds the entity described by this target, if it exists in the provided container.
    pub fn find_target_entity_in_container(
        &self,
        containing_entity: Entity,
        looking_entity: Entity,
        world: &World,
    ) -> Option<Entity> {
        //TODO take location chain into account

        if let Some(container) = world.get::<Container>(containing_entity) {
            if let Some(found_entity) =
                container.find_entity_by_name(&self.name, looking_entity, world)
            {
                return Some(found_entity);
            }
        }

        None
    }
}

/// An error while parsing input.
pub enum InputParseError {
    /// The input did not correspond to any command.
    UnknownCommand,
    /// The input was not valid for the matched command.
    CommandParseError {
        /// The name of the verb corresponding to the command.
        verb: String,
        /// The error that occurred when parsing the input as the command.
        error: CommandParseError,
    },
}

/// An error while parsing input into a specific command.
pub enum CommandParseError {
    /// A required target was not provided.
    MissingTarget,
    /// A provided target is not in the presence of the entity that provided the input.
    TargetNotFound(CommandTarget),
    /// Something else is wrong with a custom message.
    Other(String),
}

pub trait InputParser: Send + Sync {
    /// Parses input from the provided entity into an action.
    /// TODO should this be in 2 stages: first convert from the input string to some intermediate representation, then convert from that into an action?
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError>;

    /// Returns all the input formats that would cause valid actions to be produced by this parser.
    /// Targets in the provided formats are denoted with "<>".
    fn get_input_formats(&self) -> Vec<String>;

    /// Returns all the input formats that would cause valid actions to be produced by this parser if the provided entity was included as a target by the POV entity.
    /// Targets in the provided formats are denoted with "<>".
    ///
    /// For example, if this parser returns actions that act on entities with a `Location` component, then passing in an entity with that
    /// component might produce an output of `Some(["move <> to <>"])`, whereas passing in an entity without that component would produce `None`.
    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Option<Vec<String>>;
}

pub fn input_formats_if_has_component<C: Component>(
    entity: Entity,
    world: &World,
    formats: &[&str],
) -> Option<Vec<String>> {
    if world.get::<C>(entity).is_some() {
        return Some(formats.iter().map(|s| s.to_string()).collect());
    }

    None
}

fn parse_input_with<'a, I>(
    input: &str,
    source_entity: Entity,
    world: &World,
    input_parsers: I,
) -> Result<Box<dyn Action>, InputParseError>
where
    I: IntoIterator<Item = &'a Box<dyn InputParser>>,
{
    let mut errors = Vec::new();
    for parser in input_parsers {
        match parser.parse(input, source_entity, world) {
            Ok(a) => return Ok(a),
            Err(e) => errors.push(e),
        }
    }

    for error in errors {
        match error {
            InputParseError::UnknownCommand => (),
            InputParseError::CommandParseError { .. } => return Err(error),
        }
    }

    Err(InputParseError::UnknownCommand)
}
