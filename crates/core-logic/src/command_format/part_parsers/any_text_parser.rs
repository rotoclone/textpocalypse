use std::any::Any;

use bevy_ecs::prelude::*;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone, Copy)]
pub struct AnyTextParser;

impl ParsePart<String> for AnyTextParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<String> {
        // TODO how is this supposed to know when to stop?
        todo!() //TODO
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
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
    }
}
