use std::any::Any;

use bevy_ecs::prelude::*;

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

    fn as_string_for_error(
        &self,
        _: PartParserContext,
        parsed: Option<String>,
        _: &World,
    ) -> Option<String> {
        parsed
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

    fn as_string_for_error_untyped(
        &self,
        context: PartParserContext,
        parsed: Option<Box<dyn Any>>,
        world: &World,
    ) -> Option<String> {
        self.as_string_for_error(
            context,
            parsed.map(|p| {
                *p.downcast::<String>()
                    .unwrap_or_else(|e| panic!("parsed value should be a String, but was {e:?}",))
            }),
            world,
        )
    }
}
