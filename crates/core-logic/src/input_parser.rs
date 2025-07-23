use std::{collections::HashSet, fmt::Display, sync::LazyLock};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use log::debug;
use regex::Regex;

use crate::{
    action::Action,
    command_format::{CommandFormatDescription, CommandFormatParseError, PartParserContext},
    component::{Container, CustomInputParser, Location},
    found_entities::FoundEntities,
    Direction, GameMessage, StandardInputParsers,
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
///
/// Entities in `entity`'s inventory will appear first, then entities in `entity`'s location, then the location itself.
/// Within those groupings the entities will be sorted in their natural order for consistency.
/// TODO this is used to find valid entities to target for commands, so how will it work for a command like "get thing from box"?
pub fn find_entities_in_presence_of(entity: Entity, world: &World) -> Vec<Entity> {
    let location_id = world
        .get::<Location>(entity)
        .expect("Entity should have a location")
        .id;

    // include entities in the provided entity's location
    let location = world
        .get::<Container>(location_id)
        .expect("Entity's location should be a container");

    let location_entities = location.get_entities(entity, world);

    // include entities in the provided entity's inventory
    let inventory_entities = if let Some(inventory) = world.get::<Container>(entity) {
        inventory.get_entities(entity, world)
    } else {
        HashSet::new()
    };

    let mut entities = Vec::with_capacity(inventory_entities.len() + location_entities.len() + 1);
    entities.extend(inventory_entities.iter().sorted());
    entities.extend(location_entities.iter().sorted());
    entities.push(location_id);

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
        let potential_targets = self.find_target_entities(looking_entity, world);

        potential_targets
            .exact
            .first()
            .or(potential_targets.partial.first())
            .copied()
    }

    /// Finds all the possible entities described by this target, if any exist from the perspective of the looking entity.
    pub fn find_target_entities(
        &self,
        looking_entity: Entity,
        world: &World,
    ) -> FoundEntities<PortionMatched> {
        debug!("Finding {self:?} from the perspective of {looking_entity:?}");

        match self {
            CommandTarget::Myself => FoundEntities::new_single_exact(looking_entity),
            CommandTarget::Here => {
                let location_id = world
                    .get::<Location>(looking_entity)
                    .expect("Looking entity should have a location")
                    .id;
                FoundEntities::new_single_exact(location_id)
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
                    FoundEntities::new_single_exact(connecting_entity)
                } else {
                    FoundEntities::new()
                }
            }
            CommandTarget::Named(target_name) => {
                target_name.find_target_entities(looking_entity, world)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandTargetName {
    pub name: String,
    //TODO actually this should be restricted probably, since multiply-nested containers is annoying to deal with
    pub location_chain: Vec<String>,
}

impl Display for CommandTargetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO include location chain
        self.name.fmt(f)
    }
}

impl CommandTargetName {
    /// Finds all the entities described by this target, if any exist from the perspective of the looking entity.
    pub fn find_target_entities(
        &self,
        looking_entity: Entity,
        world: &World,
    ) -> FoundEntities<PortionMatched> {
        //TODO take location chain into account

        let mut found_entities = FoundEntities::new();

        // search the looking entity's inventory
        // TODO allow callers to define whether inventory or location should be searched first
        if let Some(container) = world.get::<Container>(looking_entity) {
            found_entities.extend(container.find_entities_by_name(
                &self.name,
                looking_entity,
                world,
            ));
        }

        // search the looking entity's location
        let location_id = world
            .get::<Location>(looking_entity)
            .expect("Looking entity should have a location")
            .id;
        let location = world
            .get::<Container>(location_id)
            .expect("Looking entity's location should be a container");
        found_entities.extend(location.find_entities_by_name(&self.name, looking_entity, world));

        found_entities
    }

    /// Finds all the entities described by this target, if any exist in the provided container.
    pub fn find_target_entities_in_container(
        &self,
        containing_entity: Entity,
        looking_entity: Entity,
        world: &World,
    ) -> Vec<Entity> {
        //TODO take location chain into account

        if let Some(container) = world.get::<Container>(containing_entity) {
            return container.find_entities_by_name(&self.name, looking_entity, world);
        }

        Vec::new()
    }
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
    /// TODO have this return a Vec<Vec<FormatStringPart>> or Vec<InputFormatDescription> or something
    fn get_input_formats(&self) -> Vec<String>;

    /// Returns all the input formats that would cause valid actions to be produced by this parser if the provided entity was included as a target by the POV entity.
    /// Targets in the provided formats are denoted with "<>".
    ///
    /// For example, if this parser returns actions that act on entities with a `Location` component, then passing in an entity with that
    /// component might produce an output of `Some(["move <thing> to <place>"])`, whereas passing in an entity without that component would produce `None`.
    //// TODO have this return a Vec<Vec<FormatStringPart>> or Vec<InputFormatDescription> or something
    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String>;
}

/// An error while processing input from a player.
#[derive(Debug)]
pub enum InputParseError {
    /// An error occurred while parsing the input against the command format
    CommandFormatParseError(CommandFormatParseError),
    /// An error occurred while transforming the parsed command format into an action
    Other(String),
}

impl From<String> for InputParseError {
    fn from(value: String) -> Self {
        InputParseError::Other(value)
    }
}

impl From<CommandFormatParseError> for InputParseError {
    fn from(value: CommandFormatParseError) -> Self {
        InputParseError::CommandFormatParseError(value)
    }
}

impl InputParseError {
    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self, context: PartParserContext, world: &World) -> GameMessage {
        match self {
            InputParseError::CommandFormatParseError(e) => e.into_message(context, world),
            InputParseError::Other(s) => GameMessage::Error(s),
        }
    }
}

pub fn input_formats_if_has_component<C: Component>(
    entity: Entity,
    world: &World,
    formats: &[CommandFormatDescription],
) -> Vec<String> {
    if world.get::<C>(entity).is_some() {
        return formats.iter().map(|s| s.to_string()).collect();
    }

    Vec::new()
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
        let should_return_error = match &error {
            InputParseError::CommandFormatParseError(e) => e.any_parts_matched(),
            InputParseError::Other(_) => true,
        };

        if should_return_error {
            return Err(error);
        }
    }

    // the input didn't match any parts from any parsers
    //TODO should there be another InputParseError variant for this?
    Err(CommandFormatParseError::UnmatchedInput {
        matched_parts: Vec::new(),
        unmatched: input.to_string(),
        parsed_parts: Vec::new(),
    }
    .into())
}
