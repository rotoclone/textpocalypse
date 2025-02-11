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

use super::{parsed_value::ParsedValue, CommandPartValidateError};

pub trait ParsePart<T>: ParsePartUntyped + ParsePartClone<T> {
    /// Runs this parser on the input in `context`.
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<T>;

    /// Builds a version of this parser with no generic type.
    fn as_untyped(&self) -> Box<dyn ParsePartUntyped>;
}

pub trait ParsePartUntyped: std::fmt::Debug + Send + Sync + ParsePartUntypedClone {
    /// Runs this parser on the input in `context`.
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<ParsedValue>;
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
    /// The part was missing from the input
    NotFound,
    /// The input matched multiple targets
    /// TODO include the targets
    AmbiguousInput,
    /// The part was found, but was invalid
    Invalid(CommandPartValidateError),
}

/// Attempts to match a literal from the beginning of the provided input.
/// Returns `Ok(remaining, matched)` if `input` starts with `literal` ignoring case.
fn match_literal_ignore_case<'i>(literal: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    tag_no_case(literal)(input)
}
