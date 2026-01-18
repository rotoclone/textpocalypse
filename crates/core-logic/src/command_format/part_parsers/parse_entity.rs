use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    command_format::{
        parsed_value::ParsedValue,
        parsed_value_validators::{CommandPartValidateResult, PartValidatorContext},
        EntityTargetFinderFn, PartValidationFn,
    },
    component::{Description, PortionMatched},
    found_entities::FoundEntitiesInContainer,
    input_parser::CommandTarget,
};

use super::{CommandPartParseError, CommandPartParseResult, PartParserContext};

/// Finds entities matching the input in the entering entity's inventory and the room they're in.
pub fn default_entity_target_finder(
    context: &PartParserContext,
    world: &World,
) -> FoundEntitiesInContainer<PortionMatched> {
    FoundEntitiesInContainer {
        found_entities: CommandTarget::parse(&context.input)
            .find_target_entities(context.entering_entity, world),
        //TODO does this need to be set to anything?
        searched_container: None,
    }
}

/// Parses an entity from the provided context.
/// Chooses the best target returned from `target_finder_fn`.
pub fn parse_entity(
    context: PartParserContext,
    target_finder_fn: &EntityTargetFinderFn,
    validator: Option<&PartValidationFn<Entity>>,
    world: &World,
) -> CommandPartParseResult {
    let mut best_matches: Vec<Entity> = Vec::new();
    let mut first_invalid_match = None;
    let performing_entity = context.entering_entity;
    if context.input.is_empty() {
        return CommandPartParseResult::Failure(CommandPartParseError::Unparseable {
            details: None,
        });
    }

    let found_entities = target_finder_fn(&context, world);
    let potential_targets = found_entities.found_entities;

    let sorted_targets = potential_targets.exact_matches.iter().copied().chain(
        potential_targets
            .partial_matches
            .iter()
            .sorted()
            .map(|partial_match| partial_match.entity),
    );

    for entity in sorted_targets {
        if let CommandPartValidateResult::Invalid(_) = validator
            .as_ref()
            .map(|v| {
                v(
                    &PartValidatorContext {
                        parsed_value: entity,
                        performing_entity,
                    },
                    world,
                )
            })
            .unwrap_or(CommandPartValidateResult::Valid)
        {
            if first_invalid_match.is_none() {
                first_invalid_match = Some(entity);
            }
            continue;
        }

        // entity was valid
        best_matches.push(entity);
    }

    /* TODO remove
    for entity in find_entities_in_presence_of(performing_entity, world) {
        for name in Description::get_all_ways_to_reference(entity, performing_entity, world) {
            if let Ok((extra, matched)) = match_entity_name(name, &context.input) {
                if !extra.is_empty() {
                    //
                }
                if let CommandPartValidateResult::Invalid(_) = validator
                    .as_ref()
                    .map(|v| {
                        v(
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
    */

    //TODO provide some kind of syntax for picking something other than the first one, in case there are multiple entities in the room with identical names
    if let Some(entity) = best_matches
        .first()
        // if no valid targets were found, return the first invalid one so the user will get a nice error message about why they can't target that entity
        .or(first_invalid_match.as_ref())
    {
        // matched at least one target
        CommandPartParseResult::Success(ParsedValue::Entity(*entity))
    } else {
        // matched no targets
        let searched_container_name_part = found_entities
            .searched_container
            .map(|e| {
                format!(
                    "in {}",
                    Description::get_reference_name(e, Some(context.entering_entity), world)
                )
            })
            .unwrap_or_else(|| "here".to_string());
        CommandPartParseResult::Failure(CommandPartParseError::Unparseable {
            details: Some(format!(
                "There's no '{}' {}.",
                context.input, searched_container_name_part
            )),
        })
    }
}

/* TODO
#[cfg(test)]
mod tests {
    use crate::{command_format::literal_part, test_utils::spawn_entity_in_location, Container};

    use super::*;

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
*/
