use std::cmp::Ordering;

use bevy_ecs::prelude::*;
use nom::{bytes::complete::tag, combinator::opt, sequence::preceded, IResult};

use crate::{command_format::parsed_value::ParsedValue, find_entities_in_presence_of, Description};

use super::{
    match_literal_ignore_case, CommandPartParseError, CommandPartParseResult, ParsePart,
    ParsePartUntyped, PartParserContext,
};

#[derive(Debug, Clone, Copy)]
pub struct EntityParser;

impl ParsePart<Entity> for EntityParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<Entity> {
        let mut best_matches: Vec<(Entity, &str, &str)> = Vec::new();
        for entity in find_entities_in_presence_of(context.entering_entity, world) {
            for name in Description::get_all_ways_to_reference(entity, world) {
                if let Ok((remaining, matched)) = match_entity_name(name, context.input.as_str()) {
                    // match based on which consumes the most of the input, since that's the most complete match
                    // TODO update tests
                    if let Some((_, best_remaining, _)) = best_matches.first() {
                        match remaining.len().cmp(&best_remaining.len()) {
                            Ordering::Less => {
                                best_matches.clear();
                                best_matches.push((entity, remaining, matched));
                            }
                            Ordering::Equal => best_matches.push((entity, remaining, matched)),
                            Ordering::Greater => (),
                        }
                    } else {
                        best_matches.push((entity, remaining, matched));
                    }
                }
            }
        }

        match best_matches.len().cmp(&1) {
            Ordering::Equal => {
                // matched exactly one target
                let (entity, remaining, matched) = best_matches.first().unwrap();
                CommandPartParseResult::Success {
                    parsed: *entity,
                    consumed: matched.to_string(),
                    remaining: remaining.to_string(),
                }
            }
            Ordering::Greater => {
                // matched multiple targets
                CommandPartParseResult::Failure {
                    error: CommandPartParseError::AmbiguousInput,
                    remaining: context.input,
                }
            }
            Ordering::Less => {
                // matched no targets
                CommandPartParseResult::Failure {
                    error: CommandPartParseError::NotFound,
                    remaining: context.input,
                }
            }
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
    ) -> CommandPartParseResult<Box<dyn ParsedValue>> {
        self.parse(context, world).into_generic()
    }
}

/// Matches the name of an entity, optionally preceded by "the".
fn match_entity_name<'i>(name: &str, input: &'i str) -> IResult<&'i str, &'i str> {
    preceded(opt(tag("the ")), |i| match_literal_ignore_case(name, i))(input)
}

#[cfg(test)]
mod tests {
    use crate::{move_entity, Container, Pronouns};

    use super::*;

    fn spawn_entity_in_location(id: &str, location: Entity, world: &mut World) -> Entity {
        let entity = world.spawn(build_entity_description(id)).id();
        move_entity(entity, location, world);
        entity
    }

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
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

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
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 12 name".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "entity 12 name".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name_no_remaining() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            consumed: "entity 2 name".to_string(),
            remaining: "".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            consumed: "entity 2 name".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_name_with_the() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "the entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            consumed: "the entity 2 name".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_wrong_location() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let location_2 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_2, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_name_not_at_beginning() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "it's entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::NotFound,
            remaining: "it's entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_alias() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 alias 1 and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            consumed: "entity 2 alias 1".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    #[test]
    fn parse_match_alias_with_the() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let context = PartParserContext {
            input: "the entity 2 alias 1 and stuff".to_string(),
            entering_entity: entity_1,
        };

        let expected = CommandPartParseResult::Success {
            parsed: entity_2,
            consumed: "the entity 2 alias 1".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, EntityParser.parse(context, &world));
    }

    //TODO more tests
}
