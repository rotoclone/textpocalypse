use bevy_ecs::prelude::*;

use crate::command_format::parsed_value::ParsedValue;

use super::{CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext};

/* TODO remove
#[derive(Debug)]
pub struct OptionalParser<T: Into<ParsedValue> + std::fmt::Debug + Clone>(
    pub Box<dyn ParsePart<T>>,
);

impl<T: Into<ParsedValue> + std::fmt::Debug + Clone + ParsePart<T>> Clone for OptionalParser<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

impl<T: 'static + Into<ParsedValue> + std::fmt::Debug + Clone> ParsePart<Option<T>>
    for OptionalParser<T>
{
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Option<T>> {
        match self.0.parse(context, world) {
            CommandPartParseResult::Success {
                parsed,
                consumed,
                remaining,
            } => CommandPartParseResult::Success {
                parsed: Some(parsed),
                consumed,
                remaining,
            },
            CommandPartParseResult::Failure { remaining, .. } => CommandPartParseResult::Success {
                parsed: None,
                consumed: "".to_string(),
                remaining,
            },
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

impl<T: 'static + Into<ParsedValue> + std::fmt::Debug + Clone> ParsePartUntyped
    for OptionalParser<T>
{
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<ParsedValue> {
        self.parse(context, world).into_generic()
    }
}
    */
