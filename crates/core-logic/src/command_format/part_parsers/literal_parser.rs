use std::any::Any;

use bevy_ecs::prelude::*;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone)]
pub struct LiteralParser(pub String);

impl ParsePart<String> for LiteralParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<String> {
        todo!() //TODO
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
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
    }
}
