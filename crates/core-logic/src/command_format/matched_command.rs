use bevy_ecs::prelude::*;

use crate::command_format::{
    part_parsers::CommandPartParseResult, CommandFormat, CommandFormatPart, PartParserContext,
};

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
    pub fn parse(&self, entering_entity: Entity, world: &World) -> CommandPartParseResult {
        self.part.parse(
            PartParserContext {
                input: self.matched_input.clone(),
                entering_entity,
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
