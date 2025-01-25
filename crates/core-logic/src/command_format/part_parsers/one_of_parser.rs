use std::any::Any;

use bevy_ecs::prelude::*;
use nonempty::NonEmpty;

use crate::command_format::UntypedCommandFormatPart;

use super::{
    CommandPartParseError, CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext,
};

#[derive(Debug, Clone)]
pub struct OneOfParser(pub NonEmpty<UntypedCommandFormatPart>);

impl ParsePart<Box<dyn Any>> for OneOfParser {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        let mut first_error = None;
        for part in &self.0 {
            match part.parser.parse_untyped(context.clone(), world) {
                CommandPartParseResult::Success { parsed, remaining } => {
                    return CommandPartParseResult::Success { parsed, remaining };
                }
                CommandPartParseResult::Failure { error, .. } => {
                    first_error.get_or_insert(error);
                }
            }
        }

        CommandPartParseResult::Failure {
            error: first_error.unwrap_or(CommandPartParseError::NotFound),
            remaining: context.input,
        }
    }

    fn as_string_for_error(
        &self,
        context: PartParserContext,
        parsed: Option<Box<dyn Any>>,
        world: &World,
    ) -> Option<String> {
        //TODO need to know which part actually got matched, so `as_string_for_error can be delegated to it`
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

impl ParsePartUntyped for OneOfParser {
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
        parsed: Option<Box<dyn Any>>,
        world: &World,
    ) -> Option<String> {
        self.as_string_for_error(context, parsed, world)
    }
}
