use std::any::Any;

use bevy_ecs::prelude::*;

use crate::{find_entities_in_presence_of, Description};

use super::{
    CommandPartParseError, CommandPartParseResult, ParsePart, ParsePartUntyped, PartParserContext,
};

#[derive(Debug, Clone, Copy)]
pub struct EntityParser;

impl ParsePart<Entity> for EntityParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<Entity> {
        for entity in find_entities_in_presence_of(context.entering_entity, world) {
            for name in Description::get_all_ways_to_reference(entity, world) {
                //TODO ignore case
                if let Some(remaining) = context.input.strip_prefix(name) {
                    return CommandPartParseResult::Success {
                        parsed: entity,
                        remaining: remaining.to_string(),
                    };
                }
                //TODO allow putting "the" before the entity's name
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
