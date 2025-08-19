use std::collections::HashMap;

use bevy_ecs::prelude::*;
use nom::{bytes::complete::tag_no_case, IResult};

mod parse_literal;
pub use parse_literal::parse_literal;

mod parse_any_text;
pub use parse_any_text::parse_any_text;

mod parse_entity;
pub use parse_entity::default_entity_target_finder;
pub use parse_entity::parse_entity;

mod parse_direction;
pub use parse_direction::parse_direction;

mod parse_one_of;
pub use parse_one_of::parse_one_of;

use crate::command_format::get_parsed_value;
use crate::command_format::CommandPartId;
use crate::command_format::ParsedCommandFormatPart;
use crate::command_format::UntypedCommandPartId;

use super::{parsed_value::ParsedValue, CommandPartValidateError};

/// TODO doc
#[derive(Clone)]
pub struct PartParserContext {
    pub input: String,
    pub entering_entity: Entity,
    pub parsed_parts: HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
}

impl PartParserContext {
    //TODO doc
    pub fn get_parsed_value<T: 'static>(&self, id: &CommandPartId<T>) -> Option<T>
    where
        ParsedValue: TryInto<T>,
    {
        get_parsed_value(id, &self.parsed_parts)
    }
}

//TODO doc
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseResult {
    Success(ParsedValue),
    Failure(CommandPartParseError),
}

/// An error encountered while attempting to parse a command part.
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseError {
    /// The part could not be parsed from the matched string
    Unparseable { details: Option<String> },
    /// The parsed value failed validation
    Invalid(CommandPartValidateError),
    /// A prerequisite part of this part was not matched
    PrerequisiteUnmatched(UntypedCommandPartId),
}

/// Converts `CommandPartParseResult::Success` to have a parsed value of `Option(...)`, and `CommandPartParseResult::Failure` to `CommandPartParseResult::Success` with a parsed value of `Option(None)`
pub fn parse_result_to_option(parse_result: CommandPartParseResult) -> CommandPartParseResult {
    match parse_result {
        CommandPartParseResult::Success(parsed) => {
            CommandPartParseResult::Success(ParsedValue::Option(Some(Box::new(parsed))))
        }
        CommandPartParseResult::Failure(_) => {
            CommandPartParseResult::Success(ParsedValue::Option(None))
        }
    }
}

/// Attempts to match a literal from the beginning of the provided input.
/// Returns `Ok(remaining, matched)` if `input` starts with `literal` ignoring case.
fn match_literal_ignore_case<'i>(literal: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    tag_no_case(literal)(input)
}
