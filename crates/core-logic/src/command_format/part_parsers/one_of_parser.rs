use bevy_ecs::prelude::*;
use nonempty::NonEmpty;

use crate::command_format::parsed_value::ParsedValue;

use super::{
    CommandPartParseError, CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext,
};

/* TODO remove
#[derive(Debug, Clone)]
pub struct OneOfParser(pub NonEmpty<Box<dyn ParsePartUntyped>>);

impl ParsePart<ParsedValue> for OneOfParser {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<ParsedValue> {
        let mut first_error = None;
        for parser in &self.0 {
            match parser.parse_untyped(context.clone(), world) {
                CommandPartParseResult::Success {
                    parsed,
                    consumed,
                    remaining,
                } => {
                    return CommandPartParseResult::Success {
                        parsed,
                        consumed,
                        remaining,
                    };
                }
                CommandPartParseResult::Failure { error, .. } => {
                    first_error.get_or_insert(error);
                }
            }
        }

        CommandPartParseResult::Failure {
            error: first_error.unwrap_or(CommandPartParseError::Unmatched),
            remaining: context.input,
        }
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
    ) -> CommandPartParseResult<ParsedValue> {
        self.parse(context, world).into_generic()
    }
}
    */
