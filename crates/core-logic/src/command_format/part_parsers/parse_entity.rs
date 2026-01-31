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

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        sync::{LazyLock, RwLock},
    };

    use crate::{
        command_format::{
            entity_part, part_matchers::MatchedCommandFormatPart, CommandPartId,
            CommandPartValidateError, ParsedCommandFormatPart,
        },
        found_entities::FoundEntities,
        test_utils::spawn_entity_in_location,
        Container,
    };

    use super::*;

    static CONTAINER_PART_ID: LazyLock<CommandPartId<Entity>> =
        LazyLock::new(|| CommandPartId::new("container"));

    impl PartialEq for PartValidatorContext<Entity> {
        fn eq(&self, other: &Self) -> bool {
            self.parsed_value == other.parsed_value
                && self.performing_entity == other.performing_entity
        }
    }
    impl Eq for PartValidatorContext<Entity> {}

    /// Used for ensuring the validation fn was called with the expected arguments
    static VALIDATION_FN_INFO: LazyLock<RwLock<HashSet<&'static str>>> =
        LazyLock::new(|| RwLock::new(HashSet::new()));

    /// Finds the entity with the provided name. Panics if a matching entity is not found.
    /// Can be helpful to avoid capturing variables in closures so they can be coerced into `fn` types.
    /// TODO move this to a common place probably
    fn get_entity_by_name<'w>(name: &'static str, world: &'w World) -> EntityRef<'w> {
        world
            .iter_entities()
            .find(|e| {
                world
                    .get::<Description>(e.id())
                    .map(|d| d.name == name)
                    .unwrap_or(false)
            })
            .unwrap_or_else(|| panic!("entity with name {name} should exist"))
    }

    #[test]
    fn parse_entity_empty_input() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn =
            |_, _| panic!("target finder should not be called");

        let expected =
            CommandPartParseResult::Failure(CommandPartParseError::Unparseable { details: None });

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, None, &world)
        );
    }

    #[test]
    fn parse_entity_no_match() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 12 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;

        let expected = CommandPartParseResult::Failure(CommandPartParseError::Unparseable {
            details: Some("There's no 'entity 12 name' here.".to_string()),
        });

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, None, &world)
        );
    }

    #[test]
    fn parse_entity_no_match_with_searched_container() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        spawn_entity_in_location("2", location_1, &mut world);

        let container_1 = spawn_entity_in_location("container", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 12 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: [(
                CONTAINER_PART_ID.clone().into(),
                ParsedCommandFormatPart {
                    order: 0,
                    matched_part: MatchedCommandFormatPart {
                        order: 0,
                        part: entity_part(CONTAINER_PART_ID.clone()),
                        matched_input: "something".to_string(),
                    },
                    parsed_value: ParsedValue::Entity(container_1),
                },
            )]
            .into(),
        };

        let target_finder: EntityTargetFinderFn = |context, _| FoundEntitiesInContainer {
            found_entities: FoundEntities::new(),
            searched_container: context.get_parsed_value(&CONTAINER_PART_ID),
        };

        let expected = CommandPartParseResult::Failure(CommandPartParseError::Unparseable {
            details: Some("There's no 'entity 12 name' in the entity container name.".to_string()),
        });

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, None, &world)
        );
    }

    #[test]
    fn parse_entity_only_match_is_invalid() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let expected_parsed_value = get_entity_by_name("entity 2 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            assert_eq!(
                &PartValidatorContext {
                    parsed_value: expected_parsed_value,
                    performing_entity: expected_performing_entity
                },
                context
            );
            CommandPartValidateResult::Invalid(CommandPartValidateError { details: None })
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );
    }

    #[test]
    fn parse_entity_multiple_matches_all_invalid() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("20", location_1, &mut world);
        spawn_entity_in_location("200", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let e2 = get_entity_by_name("entity 20 name", w).id();
            let e3 = get_entity_by_name("entity 200 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            let expected_context = if context.parsed_value == e2 {
                VALIDATION_FN_INFO
                    .write()
                    .unwrap()
                    .insert("parse_entity_multiple_matches_all_invalid_e2");
                PartValidatorContext {
                    parsed_value: e2,
                    performing_entity: expected_performing_entity,
                }
            } else {
                VALIDATION_FN_INFO
                    .write()
                    .unwrap()
                    .insert("parse_entity_multiple_matches_all_invalid_e3");
                PartValidatorContext {
                    parsed_value: e3,
                    performing_entity: expected_performing_entity,
                }
            };
            assert_eq!(&expected_context, context);
            CommandPartValidateResult::Invalid(CommandPartValidateError { details: None })
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );

        let expected_validation_fn_info: HashSet<&'static str> = [
            "parse_entity_multiple_matches_all_invalid_e2",
            "parse_entity_multiple_matches_all_invalid_e3",
        ]
        .into();
        assert_eq!(
            expected_validation_fn_info,
            VALIDATION_FN_INFO.read().unwrap().clone()
        );
    }

    #[test]
    fn parse_entity_exact_match_no_validation_fn() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, None, &world)
        );
    }

    #[test]
    fn parse_entity_exact_match() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let expected_parsed_value = get_entity_by_name("entity 2 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            assert_eq!(
                &PartValidatorContext {
                    parsed_value: expected_parsed_value,
                    performing_entity: expected_performing_entity
                },
                context
            );
            CommandPartValidateResult::Valid
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );
    }

    #[test]
    fn parse_entity_exact_match_with_the() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "the entity 2 name".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let expected_parsed_value = get_entity_by_name("entity 2 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            assert_eq!(
                &PartValidatorContext {
                    parsed_value: expected_parsed_value,
                    performing_entity: expected_performing_entity
                },
                context
            );
            CommandPartValidateResult::Valid
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );
    }

    #[test]
    fn parse_entity_partial_match_no_validation_fn() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 n".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, None, &world)
        );
    }

    #[test]
    fn parse_entity_partial_match() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "entity 2 n".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let expected_parsed_value = get_entity_by_name("entity 2 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            assert_eq!(
                &PartValidatorContext {
                    parsed_value: expected_parsed_value,
                    performing_entity: expected_performing_entity
                },
                context
            );
            CommandPartValidateResult::Valid
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );
    }

    #[test]
    fn parse_entity_partial_match_with_the() {
        let mut world = World::new();
        let location_1 = world.spawn(Container::new_infinite()).id();
        let entity_1 = spawn_entity_in_location("1", location_1, &mut world);
        let entity_2 = spawn_entity_in_location("2", location_1, &mut world);

        let context = PartParserContext {
            input: "the entity 2 n".to_string(),
            entering_entity: entity_1,
            parsed_parts: HashMap::new(),
        };

        let target_finder: EntityTargetFinderFn = default_entity_target_finder;
        let validation_fn: PartValidationFn<Entity> = |context, w| {
            let expected_parsed_value = get_entity_by_name("entity 2 name", w).id();
            let expected_performing_entity = get_entity_by_name("entity 1 name", w).id();
            assert_eq!(
                &PartValidatorContext {
                    parsed_value: expected_parsed_value,
                    performing_entity: expected_performing_entity
                },
                context
            );
            CommandPartValidateResult::Valid
        };

        let expected = CommandPartParseResult::Success(ParsedValue::Entity(entity_2));

        assert_eq!(
            expected,
            parse_entity(context, &target_finder, Some(&validation_fn), &world)
        );
    }

    #[test]
    fn parse_entity_one_valid_match_one_invalid_match() {
        //TODO
    }

    #[test]
    fn parse_entity_multiple_matches() {
        //TODO
    }

    /* TODO

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
    */
}
