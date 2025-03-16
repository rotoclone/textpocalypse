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

use super::{parsed_value::ParsedValue, CommandFormatPart, CommandPartValidateError};

/// TODO doc
#[derive(Clone)]
pub struct PartParserContext<'c> {
    pub input: String,
    pub entering_entity: Entity,
    pub next_part: Option<&'c CommandFormatPart>,
}

//TODO remove generic type and just make it a ParsedValue?
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseResult<T> {
    Success {
        parsed: T,
        consumed: String,
        remaining: String,
    },
    Failure {
        error: CommandPartParseError,
        remaining: String,
    },
}

impl<T: Into<ParsedValue>> CommandPartParseResult<T> {
    /// Converts the generic type on this result to `ParsedValue`, to make implementing `ParsePartUntyped` easier.
    pub fn into_generic(self) -> CommandPartParseResult<ParsedValue> {
        match self {
            CommandPartParseResult::Success {
                parsed,
                consumed,
                remaining,
            } => CommandPartParseResult::Success {
                parsed: parsed.into(),
                consumed,
                remaining,
            },
            CommandPartParseResult::Failure { error, remaining } => {
                CommandPartParseResult::Failure { error, remaining }
            }
        }
    }
}

/// An error encountered while attempting to parse a command part.
/// TODO include additional information about why a part wasn't matched, like if it's an entity that doesn't exist the error should be able to include something like "there's no <thing> here"
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseError {
    /// All the input was consumed before getting to this part
    EndOfInput,
    /// The part was not matched
    Unmatched,
    /// The part was found, but was invalid
    Invalid(CommandPartValidateError),
}

/// If the next part is a literal: returns a tuple of the input up until the literal, and the input including and after the literal.
///
/// If the next part is not a literal: returns `(input, "")`.
pub fn take_until_literal_if_next(context: PartParserContext) -> (String, String) {
    let stopping_point = if let Some(CommandFormatPart::Literal(literal, _)) = context.next_part {
        Some(literal)
    } else {
        None
    };

    take_until(context.input, stopping_point)
}

/// Splits `input` at the first instance of `stopping_point`, returning a tuple of the input before `stopping_point`, and the input including and after `stopping_point`.
/// If `stopping_point` is `None`, returns `(input, "")`.
pub fn take_until(input: impl Into<String>, stopping_point: Option<&String>) -> (String, String) {
    let input = input.into();
    if let Some(stopping_point) = stopping_point {
        let parsed = input._before(stopping_point);
        let remaining = input.strip_prefix(&parsed).unwrap_or_default();
        (parsed, remaining.to_string())
    } else {
        (input.clone(), "".to_string())
    }
}

/// Converts `CommandPartParseResult::Success` to have a parsed value of `Option(...)`, and `CommandPartParseResult::Failure` to `CommandPartParseResult::Success` with a parsed value of `Option(None)`
pub fn parse_result_to_option(
    parse_result: CommandPartParseResult<ParsedValue>,
) -> CommandPartParseResult<ParsedValue> {
    match parse_result {
        CommandPartParseResult::Success {
            parsed,
            consumed,
            remaining,
        } => CommandPartParseResult::Success {
            parsed: ParsedValue::Option(Some(Box::new(parsed))),
            consumed,
            remaining,
        },
        CommandPartParseResult::Failure {
            error: _,
            remaining,
        } => CommandPartParseResult::Success {
            parsed: ParsedValue::Option(None),
            consumed: "".to_string(),
            remaining,
        },
    }
}

/// Attempts to match a literal from the beginning of the provided input.
/// Returns `Ok(remaining, matched)` if `input` starts with `literal` ignoring case.
fn match_literal_ignore_case<'i>(literal: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    tag_no_case(literal)(input)
}
