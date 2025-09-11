use std::collections::HashMap;

use bevy_ecs::prelude::*;
use nonempty::nonempty;
use nonempty::NonEmpty;
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
    pub next_parts: Vec<&'c CommandFormatPart>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedCommandFormatPart {
    /// The index of this part in the list of parts in the format
    pub order: usize,
    /// The part that was matched
    pub part: CommandFormatPart,
    /// The portion of the input that was determined to correspond with this part
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
                next_parts: format.0.iter().skip(i + 1).collect(),
            }) {
                CommandPartMatchResult::Success { matched, remaining } => {
                    matched_parts.push(MatchedCommandFormatPart {
                        order: i,
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

enum LiteralPart<'s> {
    Single(&'s String),
    Optional(&'s String),
    OneOf(&'s NonEmpty<String>),
}

/// If the next parts are one or more consecutive `Literal`, `OptionalLiteral`, and/or `OneOfLiteral` parts: returns a tuple of the input up until the literal(s), and the input including and after the literal(s).
///
/// Otherwise: returns `(input, "")`.
pub fn take_until_literal_if_next(context: PartMatcherContext) -> (String, String) {
    let mut next_literal_parts = Vec::new();
    for next_part in context.next_parts {
        match next_part {
            CommandFormatPart::Literal(s, _) => next_literal_parts.push(LiteralPart::Single(s)),
            CommandFormatPart::OptionalLiteral(s, _) => {
                next_literal_parts.push(LiteralPart::Optional(s))
            }
            CommandFormatPart::OneOfLiteral(literals, _) => {
                next_literal_parts.push(LiteralPart::OneOf(literals))
            }
            _ => break,
        };
    }

    let permutations = generate_literal_permutations(&next_literal_parts);
    let mut best_match: Option<(String, String)> = None;

    for permutation in permutations {
        let (taken, remaining) = take_until(&context.input, Some(&permutation));
        if let Some((best_taken, _)) = &best_match {
            // "best" is considered the smallest amount of characters consumed, i.e. the first instance of the literal(s)
            if taken._count_graphemes() < best_taken._count_graphemes() {
                best_match = Some((taken, remaining));
            }
        } else {
            best_match = Some((taken, remaining));
        }
    }

    if let Some((taken, remaining)) = best_match {
        (taken, remaining)
    } else {
        (context.input, "".to_string())
    }
}

/// Generates all the valid permutations of the provided literal parts.
/// For example, if two `OneOf` parts are provided, one with "a" or "b" and one with "c" or "d", then ["ac", "ad", "bc", "bd"] will be returned.
fn generate_literal_permutations(next_literal_parts: &[LiteralPart]) -> Vec<String> {
    // generate permutations for all but the last part
    let mut permutations =
        generate_literal_permutations(&next_literal_parts[..next_literal_parts.len() - 1]);

    // now add the permutation(s) for the last part
    if let Some(last_part) = next_literal_parts.last() {
        let to_append = match last_part {
            LiteralPart::Single(s) => nonempty![s.as_str()],
            LiteralPart::Optional(s) => nonempty![s.as_str(), ""],
            LiteralPart::OneOf(literals) => literals,
        };

        if to_append.len() == 1 {
            //TODO try to do this without a special size-1 case
            permutations
                .iter_mut()
                .for_each(|mut permutation| *permutation += to_append[0]);
        } else {
            // TODO if to_append has multiple items, need to multiply the number of elements in `permutations` by its length
        }
    }
    todo!() //TODO
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
