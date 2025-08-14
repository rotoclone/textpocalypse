use std::collections::HashMap;

use bevy_ecs::prelude::*;
use voca_rs::Voca;

use crate::command_format::{
    part_parsers::CommandPartParseResult, CommandFormat, CommandFormatPart,
    ParsedCommandFormatPart, PartParserContext, UntypedCommandPartId,
};

mod match_literal;
pub use match_literal::match_literal;

mod match_one_of_literal;
pub use match_one_of_literal::match_one_of_literal;

mod match_until_next_literal;
pub use match_until_next_literal::match_until_next_literal;

mod match_direction;
pub use match_direction::match_direction;

/// TODO doc
#[derive(Clone)]
pub struct PartMatcherContext<'c> {
    pub input: String,
    pub next_part: Option<&'c CommandFormatPart>,
}

//TODO doc
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartMatchResult {
    Success {
        matched: String,
        remaining: String,
    },
    Failure {
        error: CommandPartMatchError,
        remaining: String,
    },
}

/// An error encountered while attempting to match a command part.
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartMatchError {
    /// All the input was consumed before getting to this part
    EndOfInput,
    /// The part was not matched
    Unmatched { details: Option<String> },
}

/// A part that has been associated with a portion of the input string
#[derive(Debug, Clone)]
pub struct MatchedCommandFormatPart {
    pub part: CommandFormatPart,
    pub matched_input: String,
}

impl MatchedCommandFormatPart {
    /// Parses this matched part into an actual parsed value.
    pub fn parse(
        &self,
        entering_entity: Entity,
        parsed_parts: HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
        world: &World,
    ) -> CommandPartParseResult {
        self.part.parse(
            PartParserContext {
                input: self.matched_input.clone(),
                entering_entity,
                parsed_parts,
            },
            world,
        )
    }
}

/// An intermediate state during command parsing, where some parts may have been associated with a portion of the input string, but the part(s) haven't actually been parsed yet.
pub struct MatchedCommand {
    /// The parts that were successfully matched
    pub matched_parts: Vec<MatchedCommandFormatPart>,
    /// Any parts that weren't matched
    pub unmatched_parts: Vec<CommandFormatPart>,
    /// Any remaining un-matched input
    pub remaining_input: String,
}

impl MatchedCommand {
    /// Attempts to match parts from a format to portions of the provided input.
    pub fn from_format(format: &CommandFormat, input: impl Into<String>) -> MatchedCommand {
        let mut remaining_input = input.into();
        let mut matched_parts = Vec::new();

        for (i, part) in format.0.iter().enumerate() {
            match part.match_from(PartMatcherContext {
                input: remaining_input,
                next_part: format.0.get(i + 1),
            }) {
                CommandPartMatchResult::Success { matched, remaining } => {
                    matched_parts.push(MatchedCommandFormatPart {
                        part: part.clone(),
                        matched_input: matched,
                    });

                    remaining_input = remaining;
                }
                CommandPartMatchResult::Failure { remaining, .. } => {
                    let unmatched_parts =
                        format.0.iter().skip(matched_parts.len()).cloned().collect();
                    // TODO is is ok to throw away the matching error? will it be needed later for building an error message?
                    return MatchedCommand {
                        matched_parts,
                        unmatched_parts,
                        remaining_input: remaining,
                    };
                }
            }
        }

        MatchedCommand {
            matched_parts,
            unmatched_parts: Vec::new(),
            remaining_input,
        }
    }
}

/// If the next part is a literal: returns a tuple of the input up until the literal, and the input including and after the literal.
///
/// If the next part is not a literal: returns `(input, "")`.
///
/// TODO deal with if the next part is an optional literal or a one of part with a bunch of literals
pub fn take_until_literal_if_next(context: PartMatcherContext) -> (String, String) {
    let stopping_point = if let Some(CommandFormatPart::Literal(literal, _)) = context.next_part {
        Some(literal)
    } else {
        None
    };

    take_until(context.input, stopping_point.map(|s| s.as_str()))
}

/// Splits `input` at the first instance of `stopping_point`, returning a tuple of the input before `stopping_point`, and the input including and after `stopping_point`.
/// If `stopping_point` is `None`, or `stopping_point` isn't in `input`, returns `(input, "")`.
pub fn take_until(input: impl Into<String>, stopping_point: Option<&str>) -> (String, String) {
    //TODO tests for this
    let input = input.into();
    dbg!(&input, &stopping_point); //TODO
    if let Some(stopping_point) = stopping_point {
        if !input.contains(stopping_point) {
            // `_before` returns an empty string if the provided substring isn't found, but for the purposes of this function we want the whole input in that case
            return (input.clone(), "".to_string());
        }

        let parsed = if input.starts_with(stopping_point) {
            // apparently `_before` doesn't properly handle if the string starts with the provided substring, so deal with that case manually
            // this check can be removed once https://github.com/a-merezhanyi/voca_rs/pull/27 is merged
            "".to_string()
        } else {
            input._before(stopping_point)
        };
        dbg!(&parsed); //TODO
        let remaining = input.strip_prefix(&parsed).unwrap_or_default();
        (parsed, remaining.to_string())
    } else {
        (input.clone(), "".to_string())
    }
}

/// Converts `CommandPartMatchResult::Failure` to `CommandPartMatchResult::Success` with a matched value of an empty string.
/// Doesn't touch `CommandPartMatchResult::Success`.
pub fn match_result_to_option(match_result: CommandPartMatchResult) -> CommandPartMatchResult {
    match match_result {
        CommandPartMatchResult::Success { matched, remaining } => {
            CommandPartMatchResult::Success { matched, remaining }
        }
        CommandPartMatchResult::Failure { remaining, .. } => CommandPartMatchResult::Success {
            matched: String::new(),
            remaining,
        },
    }
}
