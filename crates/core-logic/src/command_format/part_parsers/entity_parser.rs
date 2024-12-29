use std::any::Any;

use bevy_ecs::prelude::*;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone, Copy)]
pub struct EntityParser;

impl ParsePart<Entity> for EntityParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<Entity> {
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(*self)
    }
}

impl ParsePartUntyped for EntityParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
    }
}
