use bevy_ecs::prelude::*;

use crate::command_format::parsed_value::ParsedValue;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone, Copy)]
pub struct AnyTextParser;

impl ParsePart<String> for AnyTextParser {
    fn parse(&self, context: PartParserContext, _: &World) -> CommandPartParseResult<String> {
        // TODO allow somehow specifying to stop at a certain point?
        CommandPartParseResult::Success {
            parsed: context.input,
            remaining: "".to_string(),
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(*self)
    }
}

impl ParsePartUntyped for AnyTextParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn ParsedValue>> {
        self.parse(context, world).into_generic()
    }
}
