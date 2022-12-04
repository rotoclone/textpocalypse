use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::{
    action::Action,
    component::{Location, Room},
};

lazy_static! {
    static ref SELF_TARGET_PATTERN: Regex = Regex::new("^(me|myself|self)$").unwrap();
    static ref HERE_TARGET_PATTERN: Regex = Regex::new("^(here)$").unwrap();
}

pub enum CommandTargetError {
    MissingPrimaryTarget,
    MissingSecondaryTarget,
}

pub enum CommandParseError {
    WrongVerb,
    CorrectVerb(CorrectVerbError),
}

pub enum CorrectVerbError {
    Target(CommandTargetError),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandTargets {
    pub primary: Option<CommandTarget>,
    pub secondary: Option<CommandTarget>,
}

impl CommandTargets {
    /// Creates a `CommandTargets` for no targets.
    pub fn none() -> CommandTargets {
        CommandTargets {
            primary: None,
            secondary: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommandTarget {
    Myself,
    Here,
    //TODO add a Direction variant?
    Named(CommandTargetName),
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

impl CommandTargetName {
    /// Finds the entity described by this target, if it exists from the perspective of the looking entity.
    pub fn find_target_entity(&self, looking_entity: Entity, world: &World) -> Option<Entity> {
        //TODO take location chain into account
        //TODO also search the looking entity's inventory
        let location_id = world
            .get::<Location>(looking_entity)
            .expect("Looking entity should have a location")
            .id;
        let room = world
            .get::<Room>(location_id)
            .expect("Looking entity's location should be a room");
        room.find_entity_by_name(&self.name, world)
    }
}

pub enum CommandError {
    InvalidPrimaryTarget,
    InvalidSecondaryTarget,
}

pub trait Command {
    /// Converts this command to an action to be performed by the provided entity.
    fn to_action(
        &self,
        commanding_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandError>;
}

pub trait CommandParser: Send + Sync {
    /// Parses the provided input into a command.
    /// TODO does this need to be separate from the `Command` trait, or should this just return an `Action` directly?
    fn parse(&self, input: &str) -> Result<Box<dyn Command>, CommandParseError>;
}

/* TODO remove
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StandardVerb {
    #[serde(alias = "l", alias = "look at")]
    Look,
    #[serde(alias = "go", alias = "move to", alias = "go to")]
    Move,
    #[serde(alias = "grab", alias = "take", alias = "pick up")]
    Get,
    #[serde(alias = "drop", alias = "place")]
    Put,
    Open,
    #[serde(alias = "shut")]
    Close,
}
*/

pub enum InputParseError {
    MultipleMatchingCommands(Vec<Box<dyn Command>>),
    PartiallyMatchingCommands(Vec<CorrectVerbError>),
    NoMatchingCommand,
}

pub fn parse_command<'a, I>(
    input: &str,
    command_parsers: I,
) -> Result<Box<dyn Command>, InputParseError>
where
    I: IntoIterator<Item = &'a Box<dyn CommandParser>>,
{
    let mut commands = Vec::new();
    let mut errors = Vec::new();
    for parser in command_parsers {
        match parser.parse(input) {
            Ok(c) => commands.push(c),
            Err(e) => errors.push(e),
        }
    }

    if commands.len() == 1 {
        return Ok(commands.into_iter().next().unwrap());
    }

    if commands.len() > 1 {
        return Err(InputParseError::MultipleMatchingCommands(commands));
    }

    let partial_matches = errors
        .into_iter()
        .filter_map(|e| match e {
            CommandParseError::CorrectVerb(correct_verb_error) => Some(correct_verb_error),
            _ => None,
        })
        .collect::<Vec<CorrectVerbError>>();

    if !partial_matches.is_empty() {
        return Err(InputParseError::PartiallyMatchingCommands(partial_matches));
    }

    Err(InputParseError::NoMatchingCommand)
}

/* TODO remove
fn parse_verb<'i>(input: &'i str, custom_verbs: &[Box<dyn CustomVerb>]) -> IResult<&'i str, Verb> {
    //TODO but some verbs can have spaces, like "look at"
    let verb_str = take_till(|c| c == ' ')(input);

    todo!() //TODO
}
*/

/* TODO
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_verb() {
        let expected = Command {
            verb: Verb::Look,
            primary_target: None,
            secondary_target: None,
        };

        assert_eq!(Ok(("", expected)), parse_input("look"));
    }

    #[test]
    fn single_subject() {
        let expected = Command {
            verb: Verb::Look,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec![],
            }),
            secondary_target: None,
        };

        assert_for_inputs(expected, &["l thing", "look thing", "look at thing"]);
    }

    #[test]
    fn multiple_subjects() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec![],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: vec![],
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in other thing", "put thing into other thing"],
        );
    }

    #[test]
    fn multiple_subjects_with_locations() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec!["place".to_string()],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: vec!["other place".to_string()],
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in place into other thing in other place"],
        );
    }

    #[test]
    fn multiple_subjects_with_locations_ambiguous() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec![
                    "other place".to_string(),
                    "other thing".to_string(),
                    "place".to_string(),
                ],
            }),
            secondary_target: None,
        };

        assert_for_inputs(
            expected,
            &["put thing in place in other thing in other place"],
        );
    }

    #[test]
    fn multiple_subjects_quoted() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing in place".to_string(),
                location_chain: vec![],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing in other place".to_string(),
                location_chain: vec![],
            }),
        };

        assert_for_inputs(
            expected,
            &["put 'thing in place' in 'other thing in other place'"],
        );
    }

    #[test]
    fn multiple_subjects_with_multiple_locations() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec!["other place".to_string(), "place".to_string()],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: vec!["yet another place".to_string(), "another place".to_string()],
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in place in other place into other thing in another place in yet another place"],
        );
    }

    fn assert_for_inputs(expected: Command, inputs: &[&str]) {
        for input in inputs {
            assert_eq!(Ok(("", expected.clone())), parse_input(input));
        }
    }
}
*/
