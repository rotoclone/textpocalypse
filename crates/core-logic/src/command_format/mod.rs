use itertools::Itertools;
use std::{any::type_name, collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;

use nonempty::NonEmpty;

use crate::component::PortionMatched;
use crate::found_entities::FoundEntities;
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
pub use parsed_value_validators::validate_parsed_value_has_component;
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
    Direction(CommandFormatPartParams<Direction, Direction>),
    OptionalDirection(CommandFormatPartParams<Option<Direction>, Direction>),
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
            CommandFormatPart::Direction(params) => &params.options,
            CommandFormatPart::OptionalDirection(params) => &params.options,
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
            CommandFormatPart::Direction(params) => &mut params.options,
            CommandFormatPart::OptionalDirection(params) => &mut params.options,
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
            CommandFormatPart::Direction(params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalDirection(params) => {
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
            CommandFormatPart::Direction(params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
            CommandFormatPart::OptionalDirection(params) => {
                params.validator.as_ref().map(|v| genericize_validate(*v))
            }
        }
    }

    /// Sets the string to include in the error message if this part is missing (e.g. "what", "who", etc.).
    pub fn with_if_missing(mut self, s: impl Into<String>) -> Self {
        self.options_mut().if_missing = Some(s.into());
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

    /// Sets the part to never be included in error messages, regardless of if it was included in the entered command.
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
            CommandFormatPart::Direction(_) => match_direction(context),
            CommandFormatPart::OptionalDirection(_) => {
                match_result_to_option(match_direction(context))
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
            CommandFormatPart::Direction(_) => parse_direction(context),
            CommandFormatPart::OptionalDirection(_) => {
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

type EntityTargetFinderFn = fn(&PartParserContext, &World) -> FoundEntities<PortionMatched>;

/* TODO remove probably
type PartParseFn<T> = fn(PartParserContext, &World) -> Result<T, CommandPartParseError>;

type PartParseFnUntyped = Box<dyn Fn(PartParserContext, &World) -> CommandPartParseResult>;
*/

type PartValidationFn<T> = fn(PartValidatorContext<T>, &World) -> CommandPartValidateResult;

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
            PartValidatorContext {
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
    /// The string to include in the error message if this part is missing (e.g. "what", "who", etc.)
    if_missing: Option<String>,
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
                if_missing: Some(literal_string),
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
                if_missing: Some(literal_string),
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
        if_missing: Some(literal_strings.first().clone()),
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
        if_missing: Some(literal_strings.first().clone()),
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
pub fn optional_any_text_part(id: CommandPartId<Option<String>>) -> CommandFormatPart {
    build_optional_any_text_part(id, None)
}

/// Creates a part to maybe comsume any text, with a validation function.
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
    /// Adds a validation function to the part.
    pub fn with_validator(mut self, validator: PartValidationFn<Entity>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Adds a target finder function to the part.
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
pub fn optional_entity_part(id: CommandPartId<Option<Entity>>) -> CommandFormatPart {
    build_optional_entity_part(id, None, None)
}

/// Creates a part to parse an optional entity name, with a validator function.
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
pub fn direction_part(id: CommandPartId<Direction>) -> CommandFormatPart {
    build_direction_part(id, None)
}

/// Creates a part to parse a direction, with a validator function.
pub fn direction_part_with_validator(
    id: CommandPartId<Direction>,
    validator: PartValidationFn<Direction>,
) -> CommandFormatPart {
    build_direction_part(id, Some(validator))
}

fn build_direction_part(
    id: CommandPartId<Direction>,
    validator: Option<PartValidationFn<Direction>>,
) -> CommandFormatPart {
    CommandFormatPart::Direction(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part to parse an optional direction.
pub fn optional_direction_part(id: CommandPartId<Option<Direction>>) -> CommandFormatPart {
    build_optional_direction_part(id, None)
}

/// Creates a part to parse an optional direction, with a validator function.
pub fn optional_direction_part_with_validator(
    id: CommandPartId<Option<Direction>>,
    validator: PartValidationFn<Direction>,
) -> CommandFormatPart {
    build_optional_direction_part(id, Some(validator))
}

fn build_optional_direction_part(
    id: CommandPartId<Option<Direction>>,
    validator: Option<PartValidationFn<Direction>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalDirection(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
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
    /// An error occurred when attempting to match a part to a portion of the input string
    /// TODO remove?
    Matching {
        matched_parts: Vec<MatchedCommandFormatPart>,
        // Boxed to reduce size
        unmatched_parts: Box<NonEmpty<CommandFormatPart>>,
        error: CommandPartMatchError,
    },
    /// An error occurred when attempting to parse a part
    Parsing {
        parsed_parts: Vec<ParsedCommandFormatPart>,
        // Boxed to reduce size
        unparsed_parts: Box<NonEmpty<MatchedCommandFormatPart>>,
        error: CommandPartParseError,
        unmatched_parts: Vec<CommandFormatPart>,
    },
    /// Some of the input remained unmatched after matching all the parts.
    /// This error will be reported after parsing is attempted so any successfully parsed values can be used in the error message.
    UnmatchedInput {
        matched_parts: Vec<MatchedCommandFormatPart>,
        unmatched: String,
        parsed_parts: Vec<ParsedCommandFormatPart>,
    },
    /// At least one part remained unmatched after consuming all the input.
    /// This error will be reported after parsing is attempted so any successfully parsed values can be used in the error message.
    UnmatchedPart {
        matched_parts: Vec<MatchedCommandFormatPart>,
        // Boxed to reduce size
        unmatched_parts: Box<NonEmpty<CommandFormatPart>>,
        parsed_parts: Vec<ParsedCommandFormatPart>,
    },
}

impl CommandFormatParseError {
    /// Returns true if at least one part was matched, false if no parts were matched.
    pub fn any_parts_matched(&self) -> bool {
        let no_matched_parts = match self {
            CommandFormatParseError::Matching { matched_parts, .. } => matched_parts.is_empty(),
            CommandFormatParseError::Parsing { .. } => false,
            CommandFormatParseError::UnmatchedInput { matched_parts, .. } => {
                matched_parts.is_empty()
            }
            CommandFormatParseError::UnmatchedPart { matched_parts, .. } => {
                matched_parts.is_empty()
            }
        };

        !no_matched_parts
    }

    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self, context: PartParserContext, world: &World) -> GameMessage {
        if !self.any_parts_matched() {
            return GameMessage::Error("I don't understand that.".to_string());
        }

        let string = match self {
            CommandFormatParseError::Matching {
                matched_parts,
                unmatched_parts,
                error,
            } => {
                todo!() //TODO
            }
            CommandFormatParseError::Parsing {
                parsed_parts,
                unparsed_parts,
                error,
                unmatched_parts,
            } => {
                let parsed_parts_string = parsed_parts
                    .into_iter()
                    .map(|parsed_part| {
                        parsed_part.to_string_for_parse_error(context.clone(), world)
                    })
                    .join("");

                let error_detail_string = match error {
                    CommandPartParseError::Unparseable { details } => details,
                    CommandPartParseError::Invalid(error) => error.details,
                }
                .map(|message| format!(" ({message})"))
                .unwrap_or_default();

                // figure out which unparsed parts (if any) to include in the error message
                //TODO this needs to change because the parts might not be parsed in order
                let mut unparsed_parts_to_include = Vec::new();
                let mut previous_part_was_included = false;
                if unparsed_parts
                    .first()
                    .part
                    .options()
                    .include_in_errors_behavior
                    != IncludeInErrorsBehavior::Never
                {
                    // include the first unparsed part because it caused the error
                    previous_part_was_included = true;
                    unparsed_parts_to_include.push(unparsed_parts.first());
                }
                for unparsed_part in unparsed_parts.tail() {
                    let should_be_included =
                        match unparsed_part.part.options().include_in_errors_behavior {
                            IncludeInErrorsBehavior::Never => false,
                            IncludeInErrorsBehavior::OnlyIfMatched => false,
                            IncludeInErrorsBehavior::OnlyIfMatchedOrPreviousPartIncluded => {
                                previous_part_was_included
                            }
                            IncludeInErrorsBehavior::Always => true,
                        };

                    previous_part_was_included = should_be_included;

                    if should_be_included {
                        unparsed_parts_to_include.push(unparsed_part);
                    }
                }

                let unparsed_parts_string = unparsed_parts_to_include
                    .iter()
                    .map(|part| part.part.options().if_missing.as_deref().unwrap_or(""))
                    .join("");

                format!("{parsed_parts_string}{unparsed_parts_string}?{error_detail_string}")
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
            CommandFormatParseError::UnmatchedPart {
                matched_parts,
                unmatched_parts,
                parsed_parts,
            } => todo!(),
        };

        GameMessage::Error(string)
    }
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
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        let options = self.matched_part.part.options();
        if let IncludeInErrorsBehavior::Never = options.include_in_errors_behavior {
            return "".to_string();
        }

        options
            .error_string_override
            .clone()
            .unwrap_or_else(|| self.parsed_value.to_string_for_parse_error(context, world))
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
            .flat_map(|p| {
                if let Some(id) = p.part.id() {
                    Some((id, p))
                } else {
                    None
                }
            })
            .collect::<HashMap<UntypedCommandPartId, &MatchedCommandFormatPart>>();

        //TODO handle part dependencies
        for (i, part) in matched_command.matched_parts.iter().enumerate() {
            if parsed_parts_by_index.contains_key(&i) {
                // already parsed this part due to a dependency on a previous part
                continue;
            }

            let parsed_parts = parse_part(
                part,
                entering_entity,
                &matched_parts_by_id,
                &mut parsed_parts_with_ids,
                world,
            )?;
            for parsed_part in parsed_parts {
                parsed_parts_by_index.insert(i, parsed_part);
            }
        }

        if !matched_command.remaining_input.is_empty() {
            return Err(CommandFormatParseError::UnmatchedInput {
                matched_parts: matched_command.matched_parts,
                unmatched: matched_command.remaining_input,
                parsed_parts,
            });
        }

        if !matched_command.unmatched_parts.is_empty() {
            // unwrap is safe because of the `is_empty` check immediately above
            let unmatched_parts = NonEmpty::collect(matched_command.unmatched_parts).unwrap();
            return Err(CommandFormatParseError::UnmatchedPart {
                matched_parts: matched_command.matched_parts,
                unmatched_parts: Box::new(unmatched_parts),
                parsed_parts,
            });
        }

        Ok(ParsedCommand {
            parsed_parts: parsed_parts_with_ids,
        })
    }

    /// Creates a `ParsedCommand` from a list of matched parts.
    fn new(all_parsed_parts: Vec<ParsedCommandFormatPart>) -> ParsedCommand {
        let mut parsed_parts = HashMap::new();
        for parsed_part in all_parsed_parts {
            if let Some(id) = parsed_part.matched_part.part.id() {
                parsed_parts.insert(id, parsed_part);
            }
        }

        ParsedCommand { parsed_parts }
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

fn parse_part(
    matched_part: &MatchedCommandFormatPart,
    entering_entity: Entity,
    matched_parts_by_id: &HashMap<UntypedCommandPartId, &MatchedCommandFormatPart>,
    parsed_parts_with_ids: &mut HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
    world: &World,
) -> Result<Vec<ParsedCommandFormatPart>, CommandFormatParseError> {
    let mut parsed_parts = Vec::new();
    for prereq_part_id in matched_part.part.options().prerequisite_part_ids {
        if let Some(prereq_part) = matched_parts_by_id.get(&prereq_part_id) {
            parsed_parts.extend(parse_part(
                prereq_part,
                entering_entity,
                matched_parts_by_id,
                parsed_parts_with_ids,
                world,
            )?);
        } else {
            return Err(CommandFormatParseError::Parsing {
                parsed_parts: (),
                unparsed_parts: (),
                error: CommandPartParseError::PrerequisiteUnmatched),
                unmatched_parts: (),
            });
        }
        //TODO
    }

    match part.parse(entering_entity, parsed_parts_with_ids.clone(), world) {
        CommandPartParseResult::Success(parsed_value) => {
            dbg!(&part, &parsed_value); //TODO

            let parsed_part = ParsedCommandFormatPart {
                order: i,
                matched_part: part.clone(),
                parsed_value,
            };
            if let Some(id) = part.part.id() {
                parsed_parts_with_ids.insert(id, parsed_part.clone());
            }
            parsed_parts.push(parsed_part);
        }
        CommandPartParseResult::Failure(error) => {
            let mut unparsed_parts = NonEmpty::new(part.clone());
            // +1 to account for the failed part already added above
            unparsed_parts.extend(
                matched_command
                    .matched_parts
                    .iter()
                    .skip(parsed_parts.len() + 1)
                    .cloned(),
            );

            return Err(CommandFormatParseError::Parsing {
                parsed_parts,
                unparsed_parts: Box::new(unparsed_parts),
                error,
                unmatched_parts: matched_command.unmatched_parts,
            });
        }
    }
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
            .then(entity_part(CommandPartId::new("entityPartId")).with_if_missing("what"))
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
                        if_missing: None,
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
                        if_missing: Some("what".to_string()),
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
                        if_missing: None,
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
                    if_missing: None,
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
                        if_missing: None,
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
                        if_missing: None,
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
