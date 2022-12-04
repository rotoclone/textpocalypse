use bevy_ecs::prelude::*;
use nom::{bytes::complete::take_till, character::is_space, sequence::terminated, IResult};

use crate::action::Action;

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
    primary: Option<CommandTarget>,
    secondary: Option<CommandTarget>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CommandTarget {
    name: String,                // TODO should this be an entity?
    location_chain: Vec<String>, // TODO should this also be an entity?
}

pub enum CommandError {
    InvalidPrimaryTarget,
    InvalidSecondaryTarget,
}

pub trait Command {
    /// Converts this command to an action.
    fn to_action(&self, world: &World) -> Result<Box<dyn Action>, CommandError>;
}

struct LookCommand {
    targets: CommandTargets,
}

impl LookCommand {
    fn parse(input: &str) -> Result<Box<dyn Command>, CommandParseError> {
        todo!() //TODO
    }
}

impl Command for LookCommand {
    fn to_action(&self, world: &World) -> Result<Box<dyn Action>, CommandError> {
        todo!() //TODO
    }
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

type CommandParserFn = fn(&str) -> Result<Box<dyn Command>, CommandParseError>;

pub fn parse_input<'i>(
    input: &'i str,
    command_parsers: &[CommandParserFn],
) -> Result<Box<dyn Command>, InputParseError> {
    let mut commands = Vec::new();
    let mut errors = Vec::new();
    for parse_fn in command_parsers {
        match parse_fn(input) {
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
