use std::any::Any;

use bevy_ecs::prelude::*;

mod literal_parser;
pub use literal_parser::LiteralParser;

mod any_text_parser;
pub use any_text_parser::AnyTextParser;

mod entity_parser;
pub use entity_parser::EntityParser;

mod maybe_parser;
pub use maybe_parser::MaybeParser;

mod one_of_parser;
use nom::{bytes::complete::tag_no_case, IResult};
pub use one_of_parser::OneOfParser;

use super::{CommandFormatPartOptions, CommandPartValidateError};

pub trait ParsePart<T>: ParsePartUntyped + ParsePartClone<T> {
    /// Runs this parser on the input in `context`.
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<T>;

    /// Turns a parsed value into a string to include in an error message.
    /// `options` is the options on the part this parser is for.
    fn as_string_for_error(
        &self,
        context: PartParserContext,
        options: CommandFormatPartOptions,
        parsed: Option<T>,
        world: &World,
    ) -> String;

    /// Builds a version of this parser with no generic type.
    fn as_untyped(&self) -> Box<dyn ParsePartUntyped>;
}

pub trait ParsePartUntyped: std::fmt::Debug + Send + Sync + ParsePartUntypedClone {
    /// Runs this parser on the input in `context`.
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>>;

    /// Turns a parsed value into a string to include in an error message.
    fn as_string_for_error_untyped(
        &self,
        context: PartParserContext,
        options: CommandFormatPartOptions,
        parsed: Option<Box<dyn Any>>,
        world: &World,
    ) -> String;
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
pub trait ParsePartUntypedClone {
    fn clone_box(&self) -> Box<dyn ParsePartUntyped>;
}

impl<T: 'static + ParsePartUntyped + Clone> ParsePartUntypedClone for T {
    fn clone_box(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
pub trait ParsePartClone<T> {
    fn clone_box(&self) -> Box<dyn ParsePart<T>>;
}

impl<T: 'static + ParsePart<P> + Clone, P> ParsePartClone<P> for T {
    fn clone_box(&self) -> Box<dyn ParsePart<P>> {
        Box::new(self.clone())
    }
}

/// TODO doc
#[derive(Clone)]
pub struct PartParserContext {
    pub input: String,
    pub entering_entity: Entity,
}

#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseResult<T> {
    Success {
        parsed: T,
        remaining: String,
    },
    Failure {
        error: CommandPartParseError,
        remaining: String,
    },
}

impl<T: 'static> CommandPartParseResult<T> {
    /// Converts the generic type on this result to `Box<dyn Any>`, to make implementing `ParsePartUntyped` easier.
    pub fn into_generic(self) -> CommandPartParseResult<Box<dyn Any>> {
        match self {
            CommandPartParseResult::Success { parsed, remaining } => {
                CommandPartParseResult::Success {
                    parsed: Box::new(parsed),
                    remaining,
                }
            }
            CommandPartParseResult::Failure { error, remaining } => {
                CommandPartParseResult::Failure { error, remaining }
            }
        }
    }
}

/// An error encountered while attempting to parse a command part.
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartParseError {
    /// The part was missing from the input
    NotFound,
    /// The part was found, but was invalid
    Invalid(CommandPartValidateError),
}

/// Attempts to match a literal from the beginning of the provided input.
/// Returns `Ok(remaining, matched)` if `input` starts with `literal` ignoring case.
fn match_literal_ignore_case<'i>(literal: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    tag_no_case(literal)(input)
}
