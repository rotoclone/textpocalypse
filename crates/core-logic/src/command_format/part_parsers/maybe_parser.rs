use bevy_ecs::prelude::*;

use crate::{command_format::parsed_value::ParsedValue, CommandFormatPart};

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

#[derive(Debug, Clone)]
pub struct MaybeParser<T: Clone>(pub CommandFormatPart<T>);

impl<T: 'static + ParsedValue + std::fmt::Debug + Clone> ParsePart<Option<T>> for MaybeParser<T> {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Option<T>> {
        match self.0.parser.parse(context, world) {
            CommandPartParseResult::Success { parsed, remaining } => {
                CommandPartParseResult::Success {
                    parsed: Some(parsed),
                    remaining,
                }
            }
            CommandPartParseResult::Failure { remaining, .. } => CommandPartParseResult::Success {
                parsed: None,
                remaining,
            },
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(MaybeParser(self.0.clone()))
    }
}

impl<T: 'static + ParsedValue + std::fmt::Debug + Clone> ParsePartUntyped for MaybeParser<T> {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn ParsedValue>> {
        self.parse(context, world).into_generic()
    }
}
