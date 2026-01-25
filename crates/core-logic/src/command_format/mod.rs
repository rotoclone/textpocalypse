use itertools::Itertools;
use std::collections::HashSet;
use std::{any::type_name, collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;

use nonempty::NonEmpty;

use crate::component::PortionMatched;
use crate::found_entities::FoundEntitiesInContainer;
use crate::{Direction, GameMessage};

mod command_format_string;
pub use command_format_string::CommandFormatDescription;
use command_format_string::*;

mod part_matchers;
use part_matchers::*;

mod parsed_value;
pub use parsed_value::ParsedValue;

mod part_parsers;
pub use part_parsers::PartParserContext;
use part_parsers::*;

mod parsed_value_validators;
pub use parsed_value_validators::build_invalid_result;
pub use parsed_value_validators::validate_parsed_value_has_component;
pub use parsed_value_validators::validate_parsed_value_has_component_with_suffix;
pub use parsed_value_validators::CommandPartValidateError;
pub use parsed_value_validators::CommandPartValidateResult;
pub use parsed_value_validators::PartValidatorContext;

/// The format of a command a player can enter.
/// TODO change to a regular Vec instead of NonEmpty?
#[derive(Debug, PartialEq, Eq)]
pub struct CommandFormat(NonEmpty<CommandFormatPart>);

/// A `CommandPartId` with no associated type information, so different ones can be put in a collection together.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct UntypedCommandPartId(String);

impl<T> From<CommandPartId<T>> for UntypedCommandPartId {
    fn from(val: CommandPartId<T>) -> Self {
        UntypedCommandPartId(val.0)
    }
}

//TODO add a part that must be provided if another part is provided, so for example if there's an optional part that's provided it has to be preceded with a space, but if the optional part isn't provided then there can't be a space
//TODO add a part that parses into a custom type, so for example the open command doesn't need 2 separate formats (one that starts with "open" and one that starts with "close"), instead the first part could be parsed into an enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandFormatPart {
    Literal(String, CommandFormatPartParams<String, String>),
    OptionalLiteral(String, CommandFormatPartParams<Option<String>, String>),
    OneOfLiteral(NonEmpty<String>, CommandFormatPartParams<String, String>),
    OptionalOneOfLiteral(
        NonEmpty<String>,
        CommandFormatPartParams<Option<String>, String>,
    ),
    AnyText(CommandFormatPartParams<String, String>),
    OptionalAnyText(CommandFormatPartParams<Option<String>, String>),
    Entity(
        CommandFormatPartParams<Entity, Entity>,
        EntityTargetFinderFn,
    ),
    OptionalEntity(
        CommandFormatPartParams<Option<Entity>, Entity>,
        EntityTargetFinderFn,
    ),
    Direction(
        DirectionMatchMode,
        CommandFormatPartParams<Direction, Direction>,
    ),
    OptionalDirection(
        DirectionMatchMode,
        CommandFormatPartParams<Option<Direction>, Direction>,
    ),
}

impl CommandFormatPart {
    /// Gets the options for this part.
    fn options(&self) -> &CommandFormatPartOptions {
        match self {
            CommandFormatPart::Literal(_, params) => &params.options,
            CommandFormatPart::OptionalLiteral(_, params) => &params.options,
            CommandFormatPart::OneOfLiteral(_, params) => &params.options,
            CommandFormatPart::OptionalOneOfLiteral(_, params) => &params.options,
            CommandFormatPart::AnyText(params) => &params.options,
            CommandFormatPart::OptionalAnyText(params) => &params.options,
            CommandFormatPart::Entity(params, _) => &params.options,
            CommandFormatPart::OptionalEntity(params, _) => &params.options,
            CommandFormatPart::Direction(_, params) => &params.options,
            CommandFormatPart::OptionalDirection(_, params) => &params.options,
        }
    }

    /// Gets the options for this part mutably.
    fn options_mut(&mut self) -> &mut CommandFormatPartOptions {
        match self {
            CommandFormatPart::Literal(_, params) => &mut params.options,
            CommandFormatPart::OptionalLiteral(_, params) => &mut params.options,
            CommandFormatPart::OneOfLiteral(_, params) => &mut params.options,
            CommandFormatPart::OptionalOneOfLiteral(_, params) => &mut params.options,
            CommandFormatPart::AnyText(params) => &mut params.options,
            CommandFormatPart::OptionalAnyText(params) => &mut params.options,
            CommandFormatPart::Entity(params, _) => &mut params.options,
            CommandFormatPart::OptionalEntity(params, _) => &mut params.options,
            CommandFormatPart::Direction(_, params) => &mut params.options,
            CommandFormatPart::OptionalDirection(_, params) => &mut params.options,
        }
    }

    /// Gets the ID for this part, if it has one.
    pub fn id(&self) -> Option<UntypedCommandPartId> {
        match self {
            CommandFormatPart::Literal(_, params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalLiteral(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::OneOfLiteral(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::OptionalOneOfLiteral(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::AnyText(params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalAnyText(params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::Entity(params, _) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalEntity(params, _) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::Direction(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::OptionalDirection(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
        }
    }

    /// Gets the validator for this part, if it has one.
    fn validator(&self) -> Option<PartValidationFnUntyped> {
        match self {
            CommandFormatPart::Literal(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalLiteral(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OneOfLiteral(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalOneOfLiteral(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::AnyText(params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalAnyText(params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::Entity(params, _) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalEntity(params, _) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::Direction(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalDirection(_, params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
        }
    }

    /// Sets the string to include in the error message if this part is not parsed successfully (e.g. "what", "who", etc.).
    pub fn with_if_unparsed(mut self, s: impl Into<String>) -> Self {
        self.options_mut().if_unparsed = Some(s.into());
        self
    }

    /// Sets the literal string to include in the command's format string for this part (e.g. "get", "look", etc.).
    pub fn with_literal_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options_mut().format_description_part_type =
            CommandFormatDescriptionPartType::Literal(name.into());
        self
    }

    /// Sets the name of the placeholder to include in the command's format string for this part (e.g. "thing", "target", etc.).
    pub fn with_placeholder_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options_mut().format_description_part_type =
            CommandFormatDescriptionPartType::Placeholder(name.into());
        self
    }

    /// Sets the part to never be included in error messages, regardless of if it was included in the entered command.
    pub fn never_include_in_errors(mut self) -> Self {
        self.options_mut().include_in_errors_behavior = IncludeInErrorsBehavior::Never;
        self
    }

    /// Sets the part to always be included in error messages, regardless of if it was included in the entered command.
    pub fn always_include_in_errors(mut self) -> Self {
        self.options_mut().include_in_errors_behavior = IncludeInErrorsBehavior::Always;
        self
    }

    /// Sets the part to be included in error messages if the previous part was included in the error message.
    pub fn include_in_errors_if_previous_part_included(mut self) -> Self {
        self.options_mut().include_in_errors_behavior =
            IncludeInErrorsBehavior::OnlyIfMatchedOrPreviousPartIncluded;
        self
    }

    /// By default, when building an invalid command error, all the matched parts' parsed values are converted into strings to include in the error message.
    /// This overrides that behavior so `error_string` will be used instead of whatever the parsed value was.
    pub fn with_error_string_override(mut self, error_string: impl Into<String>) -> Self {
        self.options_mut().error_string_override = Some(error_string.into());
        self
    }

    /// Adds an ID to the list of parts that must be parsed before this one.
    pub fn with_prerequisite_part<T>(mut self, id: CommandPartId<T>) -> Self {
        self.options_mut().prerequisite_part_ids.push(id.into());
        self
    }

    /// Matches some of an input with the portion corresponding to this part.
    pub fn match_from(&self, context: PartMatcherContext) -> CommandPartMatchResult {
        //TODO instead of greedily matching each part in order, build a regex out of the parts as part of format building and match based on that here?
        // that would fix the issue of entity parts followed by literal space parts not properly matching entity names that include spaces
        // but it would make it harder to provide good error messages
        // maybe instead add logic to collapse multiple literal parts into one, so if the next part is " " and the part after that is oneof("a", "b") then parse until " a" or " b" instead of just " "
        match self {
            CommandFormatPart::Literal(literal, ..) => match_literal(literal, context),
            CommandFormatPart::OptionalLiteral(literal, ..) => {
                match_result_to_option(match_literal(literal, context))
            }
            CommandFormatPart::OneOfLiteral(literals, ..) => {
                match_one_of_literal(literals, context)
            }
            CommandFormatPart::OptionalOneOfLiteral(literals, ..) => {
                match_result_to_option(match_one_of_literal(literals, context))
            }
            CommandFormatPart::AnyText(_) => match_until_next_literal(context),
            CommandFormatPart::OptionalAnyText(_) => {
                match_result_to_option(match_until_next_literal(context))
            }
            CommandFormatPart::Entity(_, _) => match_until_next_literal(context),
            CommandFormatPart::OptionalEntity(_, _) => {
                match_result_to_option(match_until_next_literal(context))
            }
            CommandFormatPart::Direction(match_mode, _) => match_direction(*match_mode, context),
            CommandFormatPart::OptionalDirection(match_mode, _) => {
                match_result_to_option(match_direction(*match_mode, context))
            }
        }
    }

    /// Parses this part from some input.
    pub fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult {
        let entering_entity = context.entering_entity;
        // first parse
        let parse_result = match self {
            CommandFormatPart::Literal(..) => parse_literal(context),
            CommandFormatPart::OptionalLiteral(..) => {
                parse_result_to_option(parse_literal(context))
            }
            CommandFormatPart::OneOfLiteral(..) => parse_literal(context),
            CommandFormatPart::OptionalOneOfLiteral(..) => {
                parse_result_to_option(parse_literal(context))
            }
            CommandFormatPart::AnyText(_) => parse_any_text(context),
            CommandFormatPart::OptionalAnyText(_) => {
                parse_result_to_option(parse_any_text(context))
            }
            CommandFormatPart::Entity(params, target_finder_fn) => {
                parse_entity(context, target_finder_fn, params.validator.as_ref(), world)
            }
            CommandFormatPart::OptionalEntity(params, target_finder_fn) => parse_result_to_option(
                parse_entity(context, target_finder_fn, params.validator.as_ref(), world),
            ),
            CommandFormatPart::Direction(_, _) => parse_direction(context),
            CommandFormatPart::OptionalDirection(_, _) => {
                parse_result_to_option(parse_direction(context))
            }
        };

        // now validate
        match parse_result {
            CommandPartParseResult::Success(parsed) => {
                let validation_result = self
                    .validator()
                    .map(|v| {
                        v(
                            PartValidatorContext {
                                parsed_value: parsed.clone(),
                                performing_entity: entering_entity,
                            },
                            world,
                        )
                    })
                    .unwrap_or(CommandPartValidateResult::Valid);

                if let CommandPartValidateResult::Invalid(e) = validation_result {
                    CommandPartParseResult::Failure(CommandPartParseError::Invalid(e))
                } else {
                    CommandPartParseResult::Success(parsed)
                }
            }
            CommandPartParseResult::Failure { .. } => {
                // no need to run validator since parsing already failed
                parse_result
            }
        }
    }
}

/// Describes different matching modes for direction parts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DirectionMatchMode {
    /// Anything that's formatted like a direction should be matched (i.e. any sequence of characters followed by a space or the end of input),
    /// so invalid directions will make it to parsing and produce an error.
    Anything,
    /// Only valid directions should be matched, any other input will be considered to not match the command format at all.
    OnlyValidDirections,
}

type EntityTargetFinderFn =
    fn(&PartParserContext, &World) -> FoundEntitiesInContainer<PortionMatched>;

type PartValidationFn<T> = fn(&PartValidatorContext<T>, &World) -> CommandPartValidateResult;

type PartValidationFnUntyped =
    Box<dyn Fn(PartValidatorContext<ParsedValue>, &World) -> CommandPartValidateResult>;

fn genericize_validate<T: TryFrom<ParsedValue> + 'static>(
    f: PartValidationFn<T>,
) -> PartValidationFnUntyped {
    Box::new(move |context: PartValidatorContext<ParsedValue>, world| {
        let parsed_value = context
            .parsed_value
            .try_into()
            .unwrap_or_else(|_| panic!("parsed value should be {}", type_name::<T>()));
        f(
            &PartValidatorContext {
                parsed_value,
                performing_entity: context.performing_entity,
            },
            world,
        )
    })
}

//TODO doc
#[derive(Debug, PartialEq, Eq)]
pub struct CommandFormatPartParams<P, V> {
    id: Option<CommandPartId<P>>,
    options: CommandFormatPartOptions,
    validator: Option<PartValidationFn<V>>,
}

impl<P, V> Clone for CommandFormatPartParams<P, V> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            options: self.options.clone(),
            validator: self.validator,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct CommandFormatPartOptions {
    /// The string to include in the error message if this part isn't successfully parsed (e.g. "what", "who", etc.)
    if_unparsed: Option<String>,
    /// The string to include in the command's format description for this part (e.g. "thing", "target", etc.).
    /// If `Nothing`, the part will not be included in the format string.
    format_description_part_type: CommandFormatDescriptionPartType,
    /// When to include this part in error messages.
    include_in_errors_behavior: IncludeInErrorsBehavior,
    /// By default, when building an invalid command error, all the matched parts' parsed values are converted into strings to include in the error message.
    /// If this string is set, it will be used instead of whatever the parsed value was.
    error_string_override: Option<String>,
    /// IDs of any parts that need to be parsed before this one.
    prerequisite_part_ids: Vec<UntypedCommandPartId>,
}

/// Specifies when to include a part in an error message.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum IncludeInErrorsBehavior {
    /// The part is never included in error messages, even if it was included in the entered command.
    Never,
    /// The part is only included in an error message if it was in the entered command, or if parsing it was the cause of the error.
    #[default]
    OnlyIfMatched,
    /// The part is only included in an error message if it was in the entered command, if parsing it was the cause of the error, or if the previous part in the format was included in the error message
    OnlyIfMatchedOrPreviousPartIncluded,
    /// The part is always included in error messages, even if it was not included in the entered command.
    Always,
}

/// Creates a part to consume a literal value.
pub fn literal_part(literal: impl Into<String>) -> CommandFormatPart {
    build_literal_part(literal, None)
}

fn build_literal_part(
    literal: impl Into<String>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    let literal_string = literal.into();
    CommandFormatPart::Literal(
        literal_string.clone(),
        CommandFormatPartParams {
            id: None,
            options: CommandFormatPartOptions {
                format_description_part_type: CommandFormatDescriptionPartType::Literal(
                    literal_string.clone(),
                ),
                if_unparsed: Some(literal_string),
                ..Default::default()
            },
            validator,
        },
    )
}

/// Creates a part to maybe consume a literal value.
pub fn optional_literal_part(literal: impl Into<String>) -> CommandFormatPart {
    build_optional_literal_part(literal, None)
}

fn build_optional_literal_part(
    literal: impl Into<String>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    let literal_string = literal.into();
    CommandFormatPart::OptionalLiteral(
        literal_string.clone(),
        CommandFormatPartParams {
            id: None,
            options: CommandFormatPartOptions {
                format_description_part_type: CommandFormatDescriptionPartType::Literal(
                    literal_string.clone(),
                ),
                if_unparsed: Some(literal_string),
                ..Default::default()
            },
            validator,
        },
    )
}

/// Creates a part that consumes one of a set of possible literals.
/// Uses the first literal for the format description.
pub fn one_of_literal_part(literals: NonEmpty<impl Into<String>>) -> CommandFormatPart {
    build_one_of_literal_part(literals, None)
}

fn build_one_of_literal_part(
    literals: NonEmpty<impl Into<String>>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    let literal_strings = literals.map(|s| s.into());
    let options = CommandFormatPartOptions {
        format_description_part_type: CommandFormatDescriptionPartType::Literal(
            literal_strings.first().clone(),
        ),
        if_unparsed: Some(literal_strings.first().clone()),
        ..Default::default()
    };
    CommandFormatPart::OneOfLiteral(
        literal_strings,
        CommandFormatPartParams {
            id: None,
            options,
            validator,
        },
    )
}

/// Creates a part to maybe consume one of a set of possible literals.
/// Uses the first literal for the format description.
#[expect(unused)]
pub fn optional_one_of_literal_part(literals: NonEmpty<impl Into<String>>) -> CommandFormatPart {
    build_optional_one_of_literal_part(literals, None)
}

fn build_optional_one_of_literal_part(
    literals: NonEmpty<impl Into<String>>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    let literal_strings = literals.map(|s| s.into());
    let options = CommandFormatPartOptions {
        format_description_part_type: CommandFormatDescriptionPartType::Literal(
            literal_strings.first().clone(),
        ),
        if_unparsed: Some(literal_strings.first().clone()),
        ..Default::default()
    };
    CommandFormatPart::OptionalOneOfLiteral(
        literal_strings,
        CommandFormatPartParams {
            id: None,
            options,
            validator,
        },
    )
}

/// Creates a part to consume any text.
pub fn any_text_part(id: CommandPartId<String>) -> CommandFormatPart {
    build_any_text_part(id, None)
}

/// Creates a part to consume any text, with a validator function.
pub fn any_text_part_with_validator(
    id: CommandPartId<String>,
    validator: PartValidationFn<String>,
) -> CommandFormatPart {
    build_any_text_part(id, Some(validator))
}

fn build_any_text_part(
    id: CommandPartId<String>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    CommandFormatPart::AnyText(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part to maybe comsume any text.
#[expect(unused)]
pub fn optional_any_text_part(id: CommandPartId<Option<String>>) -> CommandFormatPart {
    build_optional_any_text_part(id, None)
}

/// Creates a part to maybe comsume any text, with a validation function.
#[expect(unused)]
pub fn optional_any_text_part_with_validator(
    id: CommandPartId<Option<String>>,
    validator: PartValidationFn<String>,
) -> CommandFormatPart {
    build_optional_any_text_part(id, Some(validator))
}

fn build_optional_any_text_part(
    id: CommandPartId<Option<String>>,
    validator: Option<PartValidationFn<String>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalAnyText(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

//TODO make something similar for optional entity parts
pub struct EntityPartBuilder {
    id: CommandPartId<Entity>,
    validator: Option<PartValidationFn<Entity>>,
    target_finder: Option<EntityTargetFinderFn>,
}

impl EntityPartBuilder {
    /// Sets the validation function of the part.
    pub fn with_validator(mut self, validator: PartValidationFn<Entity>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Sets the target finder function of the part.
    pub fn with_target_finder(mut self, target_finder: EntityTargetFinderFn) -> Self {
        self.target_finder = Some(target_finder);
        self
    }

    /// Builds the part.
    pub fn build(self) -> CommandFormatPart {
        CommandFormatPart::Entity(
            CommandFormatPartParams {
                id: Some(self.id),
                options: CommandFormatPartOptions::default(),
                validator: self.validator,
            },
            self.target_finder.unwrap_or(default_entity_target_finder),
        )
    }
}

/// Creates a builder to build a part to parse an entity name.
pub fn entity_part_builder(id: CommandPartId<Entity>) -> EntityPartBuilder {
    EntityPartBuilder {
        id,
        validator: None,
        target_finder: None,
    }
}

/// Creates a part to parse an entity name.
pub fn entity_part(id: CommandPartId<Entity>) -> CommandFormatPart {
    entity_part_builder(id).build()
}

/// Creates a part to parse an optional entity name.
#[expect(unused)]
pub fn optional_entity_part(id: CommandPartId<Option<Entity>>) -> CommandFormatPart {
    build_optional_entity_part(id, None, None)
}

/// Creates a part to parse an optional entity name, with a validator function and/or target finder function.
#[expect(unused)]
pub fn optional_entity_part_with_extras(
    id: CommandPartId<Option<Entity>>,
    validator: Option<PartValidationFn<Entity>>,
    target_finder_fn: Option<EntityTargetFinderFn>,
) -> CommandFormatPart {
    build_optional_entity_part(id, validator, target_finder_fn)
}

fn build_optional_entity_part(
    id: CommandPartId<Option<Entity>>,
    validator: Option<PartValidationFn<Entity>>,
    target_finder_fn: Option<EntityTargetFinderFn>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalEntity(
        CommandFormatPartParams {
            id: Some(id),
            options: CommandFormatPartOptions::default(),
            validator,
        },
        target_finder_fn.unwrap_or(default_entity_target_finder),
    )
}

/// Creates a part to parse a direction.
pub fn direction_part(
    id: CommandPartId<Direction>,
    match_mode: DirectionMatchMode,
) -> CommandFormatPart {
    build_direction_part(id, match_mode, None)
}

/// Creates a part to parse a direction, with a validator function.
#[expect(unused)]
pub fn direction_part_with_validator(
    id: CommandPartId<Direction>,
    match_mode: DirectionMatchMode,
    validator: PartValidationFn<Direction>,
) -> CommandFormatPart {
    build_direction_part(id, match_mode, Some(validator))
}

fn build_direction_part(
    id: CommandPartId<Direction>,
    match_mode: DirectionMatchMode,
    validator: Option<PartValidationFn<Direction>>,
) -> CommandFormatPart {
    CommandFormatPart::Direction(
        match_mode,
        CommandFormatPartParams {
            id: Some(id),
            options: CommandFormatPartOptions::default(),
            validator,
        },
    )
}

/// Creates a part to parse an optional direction.
#[expect(unused)]
pub fn optional_direction_part(
    id: CommandPartId<Option<Direction>>,
    match_mode: DirectionMatchMode,
) -> CommandFormatPart {
    build_optional_direction_part(id, match_mode, None)
}

/// Creates a part to parse an optional direction, with a validator function.
#[expect(unused)]
pub fn optional_direction_part_with_validator(
    id: CommandPartId<Option<Direction>>,
    match_mode: DirectionMatchMode,
    validator: PartValidationFn<Direction>,
) -> CommandFormatPart {
    build_optional_direction_part(id, match_mode, Some(validator))
}

fn build_optional_direction_part(
    id: CommandPartId<Option<Direction>>,
    match_mode: DirectionMatchMode,
    validator: Option<PartValidationFn<Direction>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalDirection(
        match_mode,
        CommandFormatPartParams {
            id: Some(id),
            options: CommandFormatPartOptions::default(),
            validator,
        },
    )
}

/// An identifier for a part of a command to be used to retrieve the parsed value.
/// `T` is the type that the part will be parsed into.
/// TODO change the `String` to a `&'static str` and implement `Copy`
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CommandPartId<T>(String, PhantomData<fn(T)>);

// implemting clone manually so it's implemented even if `T` is not clone
impl<T> Clone for CommandPartId<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}

impl<T> CommandPartId<T> {
    /// Creates a new part ID.
    pub fn new(value: impl Into<String>) -> CommandPartId<T> {
        CommandPartId(value.into(), PhantomData)
    }
}

impl CommandFormat {
    /// Creates a format starting with the provided part.
    pub fn new(part: CommandFormatPart) -> CommandFormat {
        CommandFormat(NonEmpty::new(part))
    }

    /// Adds a part to the format.
    /// Panics if the part has an ID and there is already a part with the same ID.
    pub fn then(mut self, part: CommandFormatPart) -> CommandFormat {
        self.add_part(part);
        self
    }

    /// Adds a part to the format.
    /// Panics if the part has an ID and there is already a part with the same ID.
    fn add_part(&mut self, part: CommandFormatPart) {
        if let Some(part_id) = part.id() {
            if self
                .0
                .iter()
                .filter_map(|existing_part| existing_part.id())
                .any(|existing_id| existing_id == part_id)
            {
                panic!("Duplicate command part ID: {}", part_id.0)
            }
        }

        self.0.push(part);
    }

    /// Gets the format description for this command format, to demonstrate how it should be used.
    pub fn get_format_description(&self) -> CommandFormatDescription {
        CommandFormatDescription::new(
            self.0
                .iter()
                .map(|part| CommandFormatDescriptionPart {
                    id: part.id().clone(),
                    part_type: part.options().format_description_part_type.clone(),
                })
                .collect(),
        )
    }
}

/// An error encountered while parsing input using a command format.
#[derive(Debug)]
pub enum CommandFormatParseError {
    /// An error occurred when attempting to parse a part
    Parsing {
        /// The processed parts, ordered by where they appear in the format.
        /// Note that parts may not be parsed in order, so there may be un-parsed parts between parsed parts.
        parts: Vec<ProcessedPart>,
        error: CommandPartParseError,
    },
    /// Some of the input remained unmatched after matching all the parts.
    /// This error will be reported after parsing is attempted so any successfully parsed values can be used in the error message.
    UnmatchedInput {
        //TODO use ProcessedPart
        matched_parts: Vec<MatchedCommandFormatPart>,
        unmatched: String,
        parsed_parts: Vec<ParsedCommandFormatPart>,
    },
    /// At least one part remained unmatched after consuming all the input.
    /// This error will be reported after parsing is attempted so any successfully parsed values can be used in the error message.
    UnmatchedPart(Vec<ProcessedPart>),
}

/// A part that was processed in some way, matched/parsed or not.
#[derive(Debug)]
pub enum ProcessedPart {
    /// The part was not matched (which also means it was not parsed)
    Unmatched(CommandFormatPart),
    /// The part was matched, but not parsed
    Matched(MatchedCommandFormatPart),
    /// The part was matched and parsed successfully
    Parsed(ParsedCommandFormatPart),
}

impl ProcessedPart {
    /// Determines if this part was successfully matched, regardless of it was parsed
    pub fn was_matched(&self) -> bool {
        matches!(self, ProcessedPart::Matched(_) | ProcessedPart::Parsed(_))
    }

    /// Determines if this part was successfully parsed
    pub fn was_parsed(&self) -> bool {
        matches!(self, ProcessedPart::Parsed(_))
    }

    /// Gets the options for the original underlying part
    pub fn options(&self) -> &CommandFormatPartOptions {
        match self {
            ProcessedPart::Unmatched(part) => part.options(),
            ProcessedPart::Matched(part) => part.part.options(),
            ProcessedPart::Parsed(part) => part.matched_part.part.options(),
        }
    }
}

impl CommandFormatParseError {
    /// Determines how many parts were successfully matched before this error occurred.
    pub fn num_parts_matched(&self) -> usize {
        match self {
            CommandFormatParseError::Parsing { parts, .. } => {
                parts.iter().filter(|part| part.was_matched()).count()
            }
            CommandFormatParseError::UnmatchedInput { matched_parts, .. } => matched_parts.len(),
            CommandFormatParseError::UnmatchedPart(parts) => {
                parts.iter().filter(|part| part.was_matched()).count()
            }
        }
    }

    /// Determines how many parts were successfully parsed before this error occurred.
    pub fn num_parts_parsed(&self) -> usize {
        match self {
            CommandFormatParseError::Parsing { parts, .. } => {
                parts.iter().filter(|part| part.was_parsed()).count()
            }
            CommandFormatParseError::UnmatchedInput { parsed_parts, .. } => parsed_parts.len(),
            CommandFormatParseError::UnmatchedPart(parts) => {
                parts.iter().filter(|part| part.was_parsed()).count()
            }
        }
    }

    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self, context: PartParserContext, world: &World) -> GameMessage {
        if self.num_parts_matched() == 0 {
            return GameMessage::Error(
                "Invalid command. Use 'help' to see valid commands.".to_string(),
            );
        }

        let string = match self {
            CommandFormatParseError::Parsing { parts, error } => {
                build_error_message_for_parts(&parts, &context, Some(error), world)
            }
            CommandFormatParseError::UnmatchedInput {
                matched_parts,
                unmatched,
                ..
            } => {
                let matched = matched_parts
                    .into_iter()
                    .map(|matched_part| matched_part.matched_input)
                    .join("");

                format!("Did you mean '{matched}' (without '{unmatched}')?")
            }
            CommandFormatParseError::UnmatchedPart(parts) => {
                build_error_message_for_parts(&parts, &context, None, world)
            }
        };

        GameMessage::Error(string)
    }
}

/// Builds an error message for the input that produced the provided parts
fn build_error_message_for_parts(
    parts: &[ProcessedPart],
    context: &PartParserContext,
    error: Option<CommandPartParseError>,
    world: &World,
) -> String {
    let mut message = String::new();
    let mut prev_part_included = false;
    for part in parts {
        let should_include = match part.options().include_in_errors_behavior {
            IncludeInErrorsBehavior::Never => false,
            //TODO this will include optional parts that weren't provided, since they successfully match an empty string...maybe there needs to be a 3rd state other than success or failure for optional parts that weren't provided
            IncludeInErrorsBehavior::OnlyIfMatched => part.was_matched(),
            IncludeInErrorsBehavior::OnlyIfMatchedOrPreviousPartIncluded => {
                part.was_matched() || prev_part_included
            }
            IncludeInErrorsBehavior::Always => true,
        };

        if !should_include {
            prev_part_included = false;
            continue;
        }

        prev_part_included = true;

        match part {
            ProcessedPart::Unmatched(unmatched_part) => {
                if let Some(s) = &unmatched_part.options().if_unparsed {
                    message += s;
                }
            }
            ProcessedPart::Matched(matched_part) => {
                // TODO determine when to include the literal matched string instead?
                if let Some(s) = &matched_part.part.options().if_unparsed {
                    message += s;
                }
            }
            ProcessedPart::Parsed(parsed_part) => {
                message += &parsed_part.to_string_for_parse_error(context, world)
            }
        }
    }

    let error_detail_string = error
        .map(|e| {
            match e {
                CommandPartParseError::Unparseable { details } => details,
                CommandPartParseError::Invalid(error) => error.details,
                CommandPartParseError::PrerequisiteUnmatched(_) => None, //TODO somehow find the part that was unmatched and turn it into an error to display?
            }
            .map(|message| format!(" ({message})"))
            .unwrap_or_default()
        })
        .unwrap_or_default();

    format!("{message}?{error_detail_string}")
}

/// A part that has been parsed into some concrete value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommandFormatPart {
    pub order: usize,
    pub matched_part: MatchedCommandFormatPart,
    pub parsed_value: ParsedValue,
}

impl PartialOrd for ParsedCommandFormatPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ParsedCommandFormatPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl ParsedCommandFormatPart {
    /// Builds a string representing this part to use in a parsing error message.
    fn to_string_for_parse_error(&self, context: &PartParserContext, world: &World) -> String {
        let options = self.matched_part.part.options();
        if let IncludeInErrorsBehavior::Never = options.include_in_errors_behavior {
            return String::new();
        }

        options.error_string_override.clone().unwrap_or_else(|| {
            self.parsed_value
                .to_string_for_parse_error(&self.matched_part.part, context, world)
        })
    }
}

pub struct ParsedCommand {
    parsed_parts: HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
}

impl ParsedCommand {
    /// Attempts to parse the matched parts from `matched_command`.
    fn from_matched_command(
        matched_command: MatchedCommand,
        entering_entity: Entity,
        world: &World,
    ) -> Result<ParsedCommand, CommandFormatParseError> {
        let mut parsed_parts_by_index = HashMap::new();
        let mut parsed_parts_with_ids = HashMap::new();
        let matched_parts_by_id = matched_command
            .matched_parts
            .iter()
            .flat_map(|p| p.part.id().map(|id| (id, p)))
            .collect::<HashMap<UntypedCommandPartId, &MatchedCommandFormatPart>>();

        let mut parse_error = None;
        for (i, part) in matched_command.matched_parts.iter().enumerate() {
            if parsed_parts_by_index.contains_key(&i) {
                // already parsed this part due to a dependency on a previous part
                continue;
            }

            if let Err(e) = parse_part(
                part,
                MatchedPartContext {
                    entering_entity,
                    matched_parts_by_id: &matched_parts_by_id,
                },
                &mut parsed_parts_with_ids,
                &mut parsed_parts_by_index,
                world,
            ) {
                if parse_error.is_none() {
                    parse_error = Some(e);
                }
            }
        }

        if let Some(error) = parse_error {
            let parts = build_processed_parts(
                matched_command.unmatched_parts.clone(),
                matched_command.matched_parts.clone(),
                &mut parsed_parts_by_index,
            );
            return Err(CommandFormatParseError::Parsing { parts, error });
        }

        if !matched_command.remaining_input.is_empty() {
            return Err(CommandFormatParseError::UnmatchedInput {
                matched_parts: matched_command.matched_parts,
                unmatched: matched_command.remaining_input,
                parsed_parts: parsed_parts_by_index.into_values().collect(),
            });
        }

        if !matched_command.unmatched_parts.is_empty() {
            let parts = build_processed_parts(
                matched_command.unmatched_parts,
                matched_command.matched_parts,
                &mut parsed_parts_by_index,
            );

            return Err(CommandFormatParseError::UnmatchedPart(parts));
        }

        Ok(ParsedCommand {
            parsed_parts: parsed_parts_with_ids,
        })
    }

    /// Gets the parsed value associated with `id`.
    /// Panics if the ID does not correspond to a part on this command, or the parsed value for this ID isn't a `T`.
    pub fn get<T: 'static>(&self, id: &CommandPartId<T>) -> T
    where
        ParsedValue: TryInto<T>,
    {
        get_parsed_value(id, &self.parsed_parts)
            .unwrap_or_else(|| panic!("No part found for ID {}", id.0))
    }
}

#[derive(Clone, Copy)]
struct MatchedPartContext<'a> {
    entering_entity: Entity,
    matched_parts_by_id: &'a HashMap<UntypedCommandPartId, &'a MatchedCommandFormatPart>,
}

/// Parses a part, but not before parsing all its prerequisite parts.
///
/// If an error is encountered, continues parsing as many parts as possible and returns the first error found.
fn parse_part(
    matched_part: &MatchedCommandFormatPart,
    matched_part_context: MatchedPartContext,
    parsed_parts_with_ids: &mut HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
    parsed_parts_by_index: &mut HashMap<usize, ParsedCommandFormatPart>,
    world: &World,
) -> Result<(), CommandPartParseError> {
    let mut part_ids_being_parsed = HashSet::new();
    if let Some(error) = parse_part_recursive(
        matched_part,
        matched_part_context,
        &mut part_ids_being_parsed,
        parsed_parts_with_ids,
        parsed_parts_by_index,
        world,
    ) {
        Err(error)
    } else {
        Ok(())
    }
}

/// Parses a part, but not before parsing all its prerequisite parts.
///
/// If an error is encountered, continues parsing as many parts as possible and returns the first error found.
fn parse_part_recursive(
    matched_part: &MatchedCommandFormatPart,
    matched_part_context: MatchedPartContext,
    part_ids_being_parsed: &mut HashSet<UntypedCommandPartId>,
    parsed_parts_with_ids: &mut HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
    parsed_parts_by_index: &mut HashMap<usize, ParsedCommandFormatPart>,
    world: &World,
) -> Option<CommandPartParseError> {
    let mut parse_error = None;
    if let Some(id) = matched_part.part.id() {
        part_ids_being_parsed.insert(id);
    }

    for prereq_part_id in &matched_part.part.options().prerequisite_part_ids {
        if part_ids_being_parsed.contains(prereq_part_id) {
            panic!(
                "Circular dependency found involving part with ID '{}'",
                prereq_part_id.0
            );
        }

        if let Some(prereq_part) = matched_part_context.matched_parts_by_id.get(prereq_part_id) {
            let error = parse_part_recursive(
                prereq_part,
                matched_part_context,
                part_ids_being_parsed,
                parsed_parts_with_ids,
                parsed_parts_by_index,
                world,
            );
            if parse_error.is_none() {
                parse_error = error;
            }
        } else if parse_error.is_none() {
            parse_error = Some(CommandPartParseError::PrerequisiteUnmatched(
                prereq_part_id.clone(),
            ));
        }
    }

    match matched_part.parse(
        matched_part_context.entering_entity,
        parsed_parts_with_ids.clone(),
        world,
    ) {
        CommandPartParseResult::Success(parsed_value) => {
            //dbg!(&matched_part, &parsed_value); //TODO

            let parsed_part = ParsedCommandFormatPart {
                order: matched_part.order,
                matched_part: matched_part.clone(),
                parsed_value,
            };
            if let Some(id) = matched_part.part.id() {
                parsed_parts_with_ids.insert(id, parsed_part.clone());
            }
            parsed_parts_by_index.insert(matched_part.order, parsed_part);
        }
        CommandPartParseResult::Failure(error) => {
            if parse_error.is_none() {
                parse_error = Some(error);
            }
        }
    }

    parse_error
}

//TODO doc
fn get_parsed_value<T: 'static>(
    id: &CommandPartId<T>,
    parsed_parts: &HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
) -> Option<T>
where
    ParsedValue: TryInto<T>,
{
    let parsed_value = parsed_parts
        .get(&UntypedCommandPartId(id.0.clone()))
        .map(|matched_part| &matched_part.parsed_value)?;

    Some(parsed_value.clone().try_into().unwrap_or_else(|_| {
        panic!(
            "Unable to convert {:?} to {}",
            parsed_value,
            type_name::<T>()
        )
    }))
}

impl CommandFormat {
    /// Attempts to parse the provided input with this format.
    pub fn parse(
        &self,
        input: impl Into<String>,
        entering_entity: Entity,
        world: &World,
    ) -> Result<ParsedCommand, CommandFormatParseError> {
        let matched_command = MatchedCommand::from_format(self, input);
        ParsedCommand::from_matched_command(matched_command, entering_entity, world)

        /* TODO
        let mut remaining_input = input.into();
        let mut has_remaining_input = true;
        let mut parsed_parts = Vec::new();

        for part in matched_parts {
            match part.parse(
                PartParserContext {
                    input: remaining_input,
                    entering_entity,
                    next_part: self.0.get(i + 1),
                },
                world,
            ) {
                CommandPartParseResult::Success {
                    parsed,
                    consumed,
                    remaining,
                } => {
                    dbg!(&parsed, &consumed, &remaining); //TODO

                    parsed_parts.push(ParsedCommandFormatPart {
                        part: part.clone(),
                        matched_input: consumed,
                        parsed_value: parsed,
                    });

                    remaining_input = remaining;
                }
                CommandPartParseResult::Failure { error, .. } => {
                    let mut unmatched_parts = NonEmpty::new(part.clone());
                    // +1 to account for the failed part already added above
                    unmatched_parts.extend(self.0.iter().skip(parsed_parts.len() + 1).cloned());

                    if !has_remaining_input {
                        // Assume that this part failed to parse due to the input being empty. This has to be down here because some parts
                        // may be optional, in which case they will parse just fine with no input, so this shouldn't pre-emptively return
                        // an end of input error without letting the part see if that's actually a problem first.
                        //TODO is it a problem to just throw away the error returned from the part?
                        return Err(CommandFormatParseError::Parsing {
                            parsed_parts,
                            unparsed_parts: Box::new(unparsed_parts),
                            error: CommandPartParseError::EndOfInput,
                        });
                    }

                    return Err(CommandFormatParseError::Parsing {
                        parsed_parts,
                        unparsed_parts: Box::new(unparsed_parts),
                        error,
                    });
                }
            }
        }

        if !remaining_input.is_empty() {
            return Err(CommandFormatParseError::UnmatchedInput {
                matched_parts,
                unmatched: remaining_input,
            });
        }

        Ok(ParsedCommand::new(parsed_parts))
        */
    }
}

/// Combines the provided unmatched, matched, and parsed parts into a single list of `ProcessedPart`s.
/// `parsed_parts_by_index` will be emptied once this returns.
fn build_processed_parts(
    unmatched_parts: Vec<CommandFormatPart>,
    matched_parts: Vec<MatchedCommandFormatPart>,
    parsed_parts_by_index: &mut HashMap<usize, ParsedCommandFormatPart>,
) -> Vec<ProcessedPart> {
    let mut parts = Vec::new();
    for (i, matched_part) in matched_parts.into_iter().enumerate() {
        if let Some(parsed_part) = parsed_parts_by_index.remove(&i) {
            parts.push(ProcessedPart::Parsed(parsed_part));
        } else {
            parts.push(ProcessedPart::Matched(matched_part));
        }
    }
    for unmatched_part in unmatched_parts {
        parts.push(ProcessedPart::Unmatched(unmatched_part));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonempty::nonempty;

    /* TODO
    impl PartialEq for CommandFormat {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl PartialEq for CommandFormatPart {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Literal(l0, l1), Self::Literal(r0, r1)) => l0 == r0 && l1 == r1,
                (Self::OptionalLiteral(l0, l1), Self::OptionalLiteral(r0, r1)) => {
                    l0 == r0 && l1 == r1
                }
                (Self::OneOfLiteral(l0, l1), Self::OneOfLiteral(r0, r1)) => l0 == r0 && l1 == r1,
                (Self::OptionalOneOfLiteral(l0, l1), Self::OptionalOneOfLiteral(r0, r1)) => {
                    l0 == r0 && l1 == r1
                }
                (Self::AnyText(l0), Self::AnyText(r0)) => l0 == r0,
                (Self::OptionalAnyText(l0), Self::OptionalAnyText(r0)) => l0 == r0,
                (Self::Entity(l0, l1), Self::Entity(r0, r1)) => l0 == r0 && l1 == r1,
                (Self::OptionalEntity(l0, l1), Self::OptionalEntity(r0, r1)) => {
                    l0 == r0 && l1 == r1
                }
                (Self::Direction(l0), Self::Direction(r0)) => l0 == r0,
                (Self::OptionalDirection(l0), Self::OptionalDirection(r0)) => l0 == r0,
                _ => false,
            }
        }
    }

    impl<P: PartialEq, V> PartialEq for CommandFormatPartParams<P, V> {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.options == other.options
        }
    }
    */

    /* TODO remove
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestValidator;

    impl ValidateParsedValue<Entity> for TestValidator {
        fn validate(
            &self,
            _: PartValidatorContext<Entity>,
            _: &World,
        ) -> CommandPartValidateResult {
            CommandPartValidateResult::Valid
        }

        fn as_untyped(&self) -> Box<dyn ValidateParsedValueUntyped> {
            Box::new(self.clone())
        }
    }

    impl ValidateParsedValueUntyped for TestValidator {
        fn validate(
            &self,
            _: PartValidatorContext<ParsedValue>,
            _: &World,
        ) -> CommandPartValidateResult {
            CommandPartValidateResult::Valid
        }
    }
    */

    #[test]
    fn format() {
        let format = CommandFormat::new(literal_part("first part"))
            .then(entity_part(CommandPartId::new("entityPartId")).with_if_unparsed("what"))
            .then(literal_part("third part"))
            .then(any_text_part(CommandPartId::new("anyTextPartId")))
            .then(optional_literal_part("optional part"))
            .then(one_of_literal_part(nonempty!["option 1", "option 2"]));

        let expected = CommandFormat(nonempty![
            CommandFormatPart::Literal(
                "first part".to_string(),
                CommandFormatPartParams {
                    id: None,
                    options: CommandFormatPartOptions {
                        if_unparsed: None,
                        format_description_part_type: CommandFormatDescriptionPartType::Literal(
                            "first part".to_string()
                        ),
                        include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                        error_string_override: None,
                        prerequisite_part_ids: Vec::new(),
                    },
                    validator: None,
                }
            ),
            CommandFormatPart::Entity(
                CommandFormatPartParams {
                    id: Some(CommandPartId::new("entityPartId")),
                    options: CommandFormatPartOptions {
                        if_unparsed: Some("what".to_string()),
                        format_description_part_type: CommandFormatDescriptionPartType::Nothing,
                        include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                        error_string_override: None,
                        prerequisite_part_ids: Vec::new(),
                    },
                    validator: None,
                },
                default_entity_target_finder
            ),
            CommandFormatPart::Literal(
                "third part".to_string(),
                CommandFormatPartParams {
                    id: None,
                    options: CommandFormatPartOptions {
                        if_unparsed: None,
                        format_description_part_type: CommandFormatDescriptionPartType::Literal(
                            "third part".to_string()
                        ),
                        include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                        error_string_override: None,
                        prerequisite_part_ids: Vec::new(),
                    },
                    validator: None
                }
            ),
            CommandFormatPart::AnyText(CommandFormatPartParams {
                id: Some(CommandPartId::new("anyTextPartId")),
                options: CommandFormatPartOptions {
                    if_unparsed: None,
                    format_description_part_type: CommandFormatDescriptionPartType::Nothing,
                    include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                    error_string_override: None,
                    prerequisite_part_ids: Vec::new(),
                },
                validator: None,
            }),
            CommandFormatPart::OptionalLiteral(
                "optional part".to_string(),
                CommandFormatPartParams {
                    id: None,
                    options: CommandFormatPartOptions {
                        if_unparsed: None,
                        format_description_part_type: CommandFormatDescriptionPartType::Literal(
                            "optional part".to_string()
                        ),
                        include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                        error_string_override: None,
                        prerequisite_part_ids: Vec::new(),
                    },
                    validator: None,
                }
            ),
            CommandFormatPart::OneOfLiteral(
                nonempty!["option 1".to_string(), "option 2".to_string()],
                CommandFormatPartParams {
                    id: None,
                    options: CommandFormatPartOptions {
                        if_unparsed: None,
                        format_description_part_type: CommandFormatDescriptionPartType::Literal(
                            "option 1".to_string()
                        ),
                        include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                        error_string_override: None,
                        prerequisite_part_ids: Vec::new(),
                    },
                    validator: None
                }
            ),
        ]);

        assert_eq!(expected, format);
    }

    #[test]
    #[should_panic = "Duplicate command part ID: somePartId"]
    fn format_duplicate_ids() {
        CommandFormat::new(literal_part("first part"))
            .then(entity_part(CommandPartId::new("somePartId")))
            .then(literal_part("third part"))
            .then(any_text_part(CommandPartId::new("somePartId")));
    }

    //TODO more tests
}
