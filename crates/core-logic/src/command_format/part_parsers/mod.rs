use bevy_ecs::prelude::*;
use nom::{bytes::complete::tag_no_case, IResult};
use voca_rs::Voca;

mod parse_literal;
pub use parse_literal::parse_literal;

mod parse_any_text;
pub use parse_any_text::parse_any_text;

mod parse_entity;
pub use parse_entity::parse_entity;

mod parse_direction;
pub use parse_direction::parse_direction;

mod parse_one_of;
pub use parse_one_of::parse_one_of;

use crate::command_format::matched_command::PartMatcherContext;

use super::{parsed_value::ParsedValue, CommandFormatPart, CommandPartValidateError};

/// TODO doc
#[derive(Clone)]
pub struct PartParserContext {
    pub input: String,
    pub entering_entity: Entity,
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

    take_until(context.input, stopping_point)
}

/// Splits `input` at the first instance of `stopping_point`, returning a tuple of the input before `stopping_point`, and the input including and after `stopping_point`.
/// If `stopping_point` is `None`, or `stopping_point` isn't in `input`, returns `(input, "")`.
pub fn take_until(input: impl Into<String>, stopping_point: Option<&String>) -> (String, String) {
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
