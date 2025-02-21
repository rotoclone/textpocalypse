use bevy_ecs::prelude::*;

use crate::command_format::parsed_value::ParsedValue;

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
                consumed: self.0.clone(),
                remaining: remaining.to_string(),
            };
        }

        CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched,
            remaining: context.input,
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

impl ParsePartUntyped for LiteralParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<ParsedValue> {
        self.parse(context, world).into_generic()
    }
}
