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
pub use one_of_parser::OneOfParser;

pub trait ParsePart<T>: ParsePartUntyped + ParsePartClone<T> {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<T>;

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped>;
}

pub trait ParsePartUntyped: std::fmt::Debug + Send + Sync + ParsePartUntypedClone {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>>;
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

pub struct PartParserContext {
    input: String,
    performing_entity: Entity,
}

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
pub enum CommandPartParseError {
    /// The part was missing from the input
    NotFound,
}
