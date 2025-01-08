use std::any::Any;

use bevy_ecs::prelude::*;
use nom::{bytes::complete::tag, combinator::opt, sequence::preceded, IResult, Parser};

use crate::{find_entities_in_presence_of, Description};

use super::{
    match_literal_ignore_case, CommandPartParseError, CommandPartParseResult, ParsePart,
    ParsePartUntyped, PartParserContext,
};

#[derive(Debug, Clone, Copy)]
pub struct EntityParser;

impl ParsePart<Entity> for EntityParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<Entity> {
        for entity in find_entities_in_presence_of(context.entering_entity, world) {
            for name in Description::get_all_ways_to_reference(entity, world) {
                if let Ok((remaining, _)) = match_entity_name(name, context.input.as_str()) {
                    return CommandPartParseResult::Success {
                        parsed: entity,
                        remaining: remaining.to_string(),
                    };
                }
                //TODO return an error if there are multiple matching entities?
            }
        }

        CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: context.input,
        }
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

/// Matches the name of an entity, optionally preceded by "the".
fn match_entity_name<'i>(name: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    preceded(opt(tag("the ")), |i| match_literal_ignore_case(name, i))(input)
}
