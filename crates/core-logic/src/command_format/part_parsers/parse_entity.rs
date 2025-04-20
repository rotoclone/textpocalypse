use std::cmp::Ordering;

use bevy_ecs::prelude::*;
use nom::{bytes::complete::tag, combinator::opt, sequence::pair, IResult};

use crate::{
    command_format::{
        parsed_value::ParsedValue,
        parsed_value_validators::{
            CommandPartValidateResult, PartValidatorContext, ValidateParsedValue,
        },
    },
    find_entities_in_presence_of, Description,
};

use super::{
    match_literal_ignore_case, take_until_literal_if_next, CommandPartParseError,
    CommandPartParseResult, PartParserContext,
};

/// Parses an entity from the provided context.
pub fn parse_entity(
    context: PartParserContext,
    validator: Option<&dyn ValidateParsedValue<Entity>>,
    world: &World,
) -> CommandPartParseResult<ParsedValue> {
    let mut best_matches: Vec<(Entity, &str, MatchedEntityName)> = Vec::new();
    let mut first_invalid_match = None;
    let performing_entity = context.entering_entity;
    let (to_parse, remaining) = take_until_literal_if_next(context);
    dbg!(&to_parse, &remaining); //TODO
    if to_parse.is_empty() {
        return CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched { details: None },
            remaining,
        };
    }

    for entity in find_entities_in_presence_of(performing_entity, world) {
        for name in Description::get_all_ways_to_reference(entity, world) {
            if let Ok((extra, matched)) = match_entity_name(name, &to_parse) {
                if let CommandPartValidateResult::Invalid(_) = validator
                    .map(|v| {
                        ValidateParsedValue::validate(
                            v,
                            PartValidatorContext {
                                parsed_value: entity,
                                performing_entity,
                            },
                            world,
                        )
                    })
                    .unwrap_or(CommandPartValidateResult::Valid)
                {
                    if first_invalid_match.is_none() {
                        first_invalid_match = Some((entity, extra, matched));
                    }
                    continue;
                }

                // match based on which consumes the most of the input, since that's the most complete match
                // TODO update tests
                if let Some((_, best_extra, _)) = best_matches.first() {
                    match extra.len().cmp(&best_extra.len()) {
                        Ordering::Less => {
                            best_matches.clear();
                            best_matches.push((entity, extra, matched));
                        }
                        Ordering::Equal => best_matches.push((entity, extra, matched)),
                        Ordering::Greater => (),
                    }
                } else {
                    best_matches.push((entity, extra, matched));
                }
            }
        }
    }

    //TODO provide some kind of syntax for picking something other than the first one, in case there are multiple entities in the room with identical names
    if let Some((entity, extra, matched)) = best_matches
        .first()
        // if no valid targets were found, return the first invalid one so the user will get a nice error message about why they can't target that entity
        .or(first_invalid_match.as_ref())
    {
        // matched at least one target
        CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(*entity),
            consumed: format!("{}{}", matched.prefix.unwrap_or_default(), matched.name),
            remaining: format!("{extra}{remaining}"),
        }
    } else {
        // matched no targets
        CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some(format!("There's no '{to_parse}' here.")),
            },
            // re-combine input string to undo split from earlier
            remaining: format!("{to_parse}{remaining}"),
        }
    }
}

struct MatchedEntityName<'a> {
    prefix: Option<&'a str>,
    name: &'a str,
}

/// Matches the name of an entity, optionally preceded by "the".
fn match_entity_name<'i>(name: &str, input: &'i str) -> IResult<&'i str, MatchedEntityName<'i>> {
    //TODO allow partial matches (i.e. "trou" would match "trousers" if it's unambiguous)
    let (remaining, (prefix, matched)) =
        pair(opt(tag("the ")), |i| match_literal_ignore_case(name, i))(input)?;

    Ok((
        remaining,
        MatchedEntityName {
            prefix,
            name: matched,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{
        command_format::{literal_part, CommandFormatPart},
        move_entity, Container, Pronouns,
    };

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
            next_part: None,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched { details: None },
            remaining: "".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'entity 12 name' here.".to_string()),
            },
            remaining: "entity 12 name".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_no_match_input_ends_with_next_literal_part() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let next_part = literal_part(" 12 name".to_string());
        let context = PartParserContext {
            input: "entity 12 name".to_string(),
            entering_entity: entity_1,
            next_part: Some(&next_part),
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'entity' here.".to_string()),
            },
            remaining: "entity 12 name".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_no_match_input_contains_next_literal_part() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let next_part = literal_part(" 12 ".to_string());
        let context = PartParserContext {
            input: "entity 12 name".to_string(),
            entering_entity: entity_1,
            next_part: Some(&next_part),
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'entity' here.".to_string()),
            },
            remaining: "entity 12 name".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "entity 2 name".to_string(),
            remaining: "".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "entity 2 name".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_match_name_remaining_in_next_literal() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let next_part = literal_part(" and stuff".to_string());
        let context = PartParserContext {
            input: "entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
            next_part: Some(&next_part),
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "entity 2 name".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "the entity 2 name".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_would_match_but_literal_next_part_is_entity_name() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let next_part = literal_part("entity 2 name".to_string());
        let context = PartParserContext {
            input: "entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
            next_part: Some(&next_part),
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched { details: None },
            remaining: "entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_would_match_but_literal_next_part_is_end_of_entity_name() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("3", location_1, &mut world);

        let next_part = literal_part(" 2 name".to_string());
        let context = PartParserContext {
            input: "entity 2 name and stuff".to_string(),
            entering_entity: entity_1,
            next_part: Some(&next_part),
        };

        //TODO this should actually be a match as long as it's unambiguous, and if it is ambiguous (which it is here), it should have a different message which includes the possible matches
        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'entity' here.".to_string()),
            },
            remaining: "entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'entity 2 name and stuff' here.".to_string()),
            },
            remaining: "entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some("There's no 'it's entity 2 name and stuff' here.".to_string()),
            },
            remaining: "it's entity 2 name and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "entity 2 alias 1".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
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
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "the entity 2 alias 1".to_string(),
            remaining: " and stuff".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    #[test]
    fn parse_match_ambiguous() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name".to_string(),
            entering_entity: entity_1,
            next_part: None,
        };

        let expected = CommandPartParseResult::Success {
            parsed: ParsedValue::Entity(entity_2),
            consumed: "entity 2 name".to_string(),
            remaining: "".to_string(),
        };

        assert_eq!(expected, parse_entity(context, None, &world));
    }

    //TODO more tests
}
