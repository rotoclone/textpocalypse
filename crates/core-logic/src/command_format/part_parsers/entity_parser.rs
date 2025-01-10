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

#[cfg(test)]
mod tests {
    use crate::Pronouns;

    use super::*;

    //TODO entities need to be in the same location
    fn build_entity_description(id: &str) -> Description {
        Description {
            name: format!("entity {id} name"),
            room_name: format!("entity {id} room name"),
            plural_name: format!("entity {id} plural name"),
            article: None,
            pronouns: Pronouns::it(),
            aliases: vec![
                format!("entity {id} alias 1"),
                format!("entity {id} alias 2"),
            ],
            description: format!("entity {id} description"),
            attribute_describers: Vec::new(),
        }
    }

    #[test]
    fn parse_empty_input() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        world.spawn(build_entity_description("2"));

        let context = PartParserContext {
            input: "".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_no_match() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        world.spawn(build_entity_description("2"));

        let context = PartParserContext {
            input: "entity 12".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "entity 12".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name_no_remaining() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        let entity_2 = world.spawn(build_entity_description("2")).id();
        world.spawn(build_entity_description("3"));

        let context = PartParserContext {
            input: "entity 2".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            remaining: "".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        let entity_2 = world.spawn(build_entity_description("2")).id();
        world.spawn(build_entity_description("3"));

        let context = PartParserContext {
            input: "entity 2 and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name_with_the() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        let entity_2 = world.spawn(build_entity_description("2")).id();
        world.spawn(build_entity_description("3"));

        let context = PartParserContext {
            input: "the entity 2 and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_name_not_at_beginning() {
        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_description("1")).id();
        world.spawn(build_entity_description("2"));
        world.spawn(build_entity_description("3"));

        let context = PartParserContext {
            input: "it's entity 2 and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "it's entity 2 and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    //TODO more tests
}
