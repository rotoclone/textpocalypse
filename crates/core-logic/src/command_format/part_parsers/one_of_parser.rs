use std::any::Any;

use bevy_ecs::prelude::*;
use nonempty::NonEmpty;

use crate::command_format::UntypedCommandFormatPart;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone)]
pub struct OneOfParser(pub NonEmpty<UntypedCommandFormatPart>);

impl ParsePart<Box<dyn Any>> for OneOfParser {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
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
}
