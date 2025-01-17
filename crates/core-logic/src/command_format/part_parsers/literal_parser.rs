use std::any::Any;

use bevy_ecs::prelude::*;

use crate::CommandFormatPartOptions;

use super::{
    CommandPartParseError, CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext,
};

//TODO allow ignoring case?
#[derive(Debug, Clone)]
pub struct LiteralParser(pub String);

impl ParsePart<String> for LiteralParser {
    fn parse(&self, context: PartParserContext, _: &World) -> CommandPartParseResult<String> {
        if let Some(remaining) = context.input.strip_prefix(&self.0) {
            return CommandPartParseResult::Success {
                parsed: self.0.clone(),
                remaining: remaining.to_string(),
            };
        }

        CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: context.input,
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }

    fn as_string_for_error(
        &self,
        _: PartParserContext,
        _: CommandFormatPartOptions,
        _: Option<String>,
        _: &World,
    ) -> String {
        self.0.clone()
    }
}

impl ParsePartUntyped for LiteralParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
    }

    fn as_string_for_error_untyped(
        &self,
        context: PartParserContext,
        options: CommandFormatPartOptions,
        parsed: Option<Box<dyn Any>>,
        world: &World,
    ) -> String {
        self.as_string_for_error(
            context,
            options,
            parsed.map(|p| *p.downcast().expect("parsed value should be a String")),
            world,
        )
    }
}
