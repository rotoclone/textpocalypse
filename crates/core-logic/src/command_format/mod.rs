use itertools::Itertools;
use std::{any::type_name, collections::HashMap, marker::PhantomData, ops::Deref};

use bevy_ecs::prelude::*;

use nonempty::NonEmpty;

use crate::{Direction, GameMessage};

mod command_format_string;
use command_format_string::*;

mod parsed_value;
use parsed_value::*;

mod part_parsers;
pub use part_parsers::PartParserContext;
use part_parsers::*;

mod parsed_value_validators;
use parsed_value_validators::*;

/// The format of a command a player can enter.
/// TODO change to a regular Vec instead of NonEmpty?
#[derive(Debug)]
pub struct CommandFormat(NonEmpty<CommandFormatPart>);

/// A `CommandPartId` with no associated type information, so different ones can be put in a collection together.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct UntypedCommandPartId(String);

impl<T> From<CommandPartId<T>> for UntypedCommandPartId {
    fn from(val: CommandPartId<T>) -> Self {
        UntypedCommandPartId(val.0)
    }
}

#[derive(Debug, Clone)]
pub enum CommandFormatPart {
    Literal(String, CommandFormatPartParams<String, String>),
    OptionalLiteral(String, CommandFormatPartParams<Option<String>, String>),
    AnyText(CommandFormatPartParams<String, String>),
    //TODO empty input already parses successfully for an AnyText part, so is OptionalAnyText necessary?
    OptionalAnyText(CommandFormatPartParams<Option<String>, String>),
    Entity(CommandFormatPartParams<Entity, Entity>),
    OptionalEntity(CommandFormatPartParams<Option<Entity>, Entity>),
    Direction(CommandFormatPartParams<Direction, Direction>),
    OptionalDirection(CommandFormatPartParams<Option<Direction>, Direction>),
    OneOf(Vec<CommandFormatPart>, CommandFormatPartOptions),
}

impl CommandFormatPart {
    /// Gets the options for this part.
    fn options(&self) -> &CommandFormatPartOptions {
        match self {
            CommandFormatPart::Literal(_, params) => &params.options,
            CommandFormatPart::OptionalLiteral(_, params) => &params.options,
            CommandFormatPart::AnyText(params) => &params.options,
            CommandFormatPart::OptionalAnyText(params) => &params.options,
            CommandFormatPart::Entity(params) => &params.options,
            CommandFormatPart::OptionalEntity(params) => &params.options,
            CommandFormatPart::Direction(params) => &params.options,
            CommandFormatPart::OptionalDirection(params) => &params.options,
            CommandFormatPart::OneOf(_, options) => options,
        }
    }

    /// Gets the options for this part mutably.
    fn options_mut(&mut self) -> &mut CommandFormatPartOptions {
        match self {
            CommandFormatPart::Literal(_, params) => &mut params.options,
            CommandFormatPart::OptionalLiteral(_, params) => &mut params.options,
            CommandFormatPart::AnyText(params) => &mut params.options,
            CommandFormatPart::OptionalAnyText(params) => &mut params.options,
            CommandFormatPart::Entity(params) => &mut params.options,
            CommandFormatPart::OptionalEntity(params) => &mut params.options,
            CommandFormatPart::Direction(params) => &mut params.options,
            CommandFormatPart::OptionalDirection(params) => &mut params.options,
            CommandFormatPart::OneOf(_, options) => options,
        }
    }

    /// Gets all the IDs associated with this part.
    /// This will usually return 0 or 1 ID, but `OneOf` parts can have more than 1.
    pub fn all_ids(&self) -> Vec<UntypedCommandPartId> {
        match self {
            CommandFormatPart::Literal(_, params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::OptionalLiteral(_, params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::AnyText(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::OptionalAnyText(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::Entity(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::OptionalEntity(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::Direction(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::OptionalDirection(params) => params
                .id
                .as_ref()
                .map(|id| vec![id.clone().into()])
                .unwrap_or_default(),
            CommandFormatPart::OneOf(parts, _) => {
                parts.iter().flat_map(|part| part.all_ids()).collect()
            }
        }
    }

    /// Gets the ID for this part, if it has one.
    /// This will always return `None` for `OneOf` parts.
    pub fn id(&self) -> Option<UntypedCommandPartId> {
        match self {
            CommandFormatPart::Literal(_, params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalLiteral(_, params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::AnyText(params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalAnyText(params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::Entity(params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalEntity(params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::Direction(params) => params.id.as_ref().map(|id| id.clone().into()),
            CommandFormatPart::OptionalDirection(params) => {
                params.id.as_ref().map(|id| id.clone().into())
            }
            CommandFormatPart::OneOf(_, _) => None,
        }
    }

    /// Gets the validator for this part, if it has one.
    /// This will always return `None` for `OneOf` parts.
    fn validator(&self) -> Option<Box<dyn ValidateParsedValueUntyped>> {
        match self {
            CommandFormatPart::Literal(_, params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::OptionalLiteral(_, params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::AnyText(params) => params.validator.as_ref().map(|v| v.as_untyped()),
            CommandFormatPart::OptionalAnyText(params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::Entity(params) => params.validator.as_ref().map(|v| v.as_untyped()),
            CommandFormatPart::OptionalEntity(params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::Direction(params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::OptionalDirection(params) => {
                params.validator.as_ref().map(|v| v.as_untyped())
            }
            CommandFormatPart::OneOf(_, _) => None,
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

    /// By default, when building an invalid command error, all the matched parts' parsed values are converted into strings to include in the error message.
    /// This overrides that behavior so `error_string` will be used instead of whatever the parsed value was.
    pub fn with_error_string_override(mut self, error_string: impl Into<String>) -> Self {
        self.options_mut().error_string_override = Some(error_string.into());
        self
    }

    /// TODO doc
    pub fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult {
        let entering_entity = context.entering_entity;
        // first parse
        let parse_result = match self {
            CommandFormatPart::Literal(literal, _) => parse_literal(literal, context),
            CommandFormatPart::OptionalLiteral(literal, _) => {
                parse_result_to_option(parse_literal(literal, context))
            }
            CommandFormatPart::AnyText(_) => parse_any_text(context),
            CommandFormatPart::OptionalAnyText(_) => {
                parse_result_to_option(parse_any_text(context))
            }
            CommandFormatPart::Entity(params) => {
                parse_entity(context, params.validator.as_deref(), world)
            }
            CommandFormatPart::OptionalEntity(params) => {
                parse_result_to_option(parse_entity(context, params.validator.as_deref(), world))
            }
            CommandFormatPart::Direction(_) => parse_direction(context),
            CommandFormatPart::OptionalDirection(_) => {
                parse_result_to_option(parse_direction(context))
            }
            CommandFormatPart::OneOf(parts, _) => parse_one_of(parts, context, world),
        };

        // now validate
        match parse_result {
            CommandPartParseResult::Success {
                parsed,
                consumed,
                remaining,
            } => {
                let validation_result = self
                    .validator()
                    .map(|v| {
                        v.validate(
                            PartValidatorContext {
                                parsed_value: parsed.clone(),
                                performing_entity: entering_entity,
                            },
                            world,
                        )
                    })
                    .unwrap_or(CommandPartValidateResult::Valid);

                if let CommandPartValidateResult::Invalid(e) = validation_result {
                    CommandPartParseResult::Failure {
                        error: CommandPartParseError::Invalid(e),
                        // re-combine these to effectively un-do the consumption since it's invalid
                        remaining: format!("{consumed}{remaining}"),
                    }
                } else {
                    CommandPartParseResult::Success {
                        parsed,
                        consumed,
                        remaining,
                    }
                }
            }
            CommandPartParseResult::Failure { .. } => {
                // no need to run validator since parsing already failed
                parse_result
            }
        }
    }
}

#[derive(Debug)]
pub struct CommandFormatPartParams<P, V> {
    id: Option<CommandPartId<P>>,
    options: CommandFormatPartOptions,
    validator: Option<Box<dyn ValidateParsedValue<V>>>,
}

impl<P, V> Clone for CommandFormatPartParams<P, V> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            options: self.options.clone(),
            validator: self
                .validator
                .as_ref()
                .map(|v| ValidateParsedValueClone::clone_box(v.deref())),
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
}

/// Specifies when to include a part in an error message.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum IncludeInErrorsBehavior {
    /// The part is never included in error messages, even if it was included in the entered command.
    Never,
    /// The part is only included in an error message if it was in the entered command, or if parsing it was the cause of the error.
    #[default]
    OnlyIfMatched,
}

/// Creates a part to consume a literal value.
pub fn literal_part(literal: impl Into<String>) -> CommandFormatPart {
    build_literal_part(literal, None)
}

fn build_literal_part(
    literal: impl Into<String>,
    validator: Option<Box<dyn ValidateParsedValue<String>>>,
) -> CommandFormatPart {
    let literal_string = literal.into();
    CommandFormatPart::Literal(
        literal_string.clone(),
        CommandFormatPartParams {
            id: None,
            options: CommandFormatPartOptions {
                format_description_part_type: CommandFormatDescriptionPartType::Literal(
                    literal_string,
                ),
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
    validator: Option<Box<dyn ValidateParsedValue<String>>>,
) -> CommandFormatPart {
    let literal_string = literal.into();
    CommandFormatPart::OptionalLiteral(
        literal_string.clone(),
        CommandFormatPartParams {
            id: None,
            options: CommandFormatPartOptions {
                format_description_part_type: CommandFormatDescriptionPartType::Literal(
                    literal_string,
                ),
                ..Default::default()
            },
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
    validator: Box<dyn ValidateParsedValue<String>>,
) -> CommandFormatPart {
    build_any_text_part(id, Some(validator))
}

fn build_any_text_part(
    id: CommandPartId<String>,
    validator: Option<Box<dyn ValidateParsedValue<String>>>,
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
    validator: Box<dyn ValidateParsedValue<String>>,
) -> CommandFormatPart {
    build_optional_any_text_part(id, Some(validator))
}

fn build_optional_any_text_part(
    id: CommandPartId<Option<String>>,
    validator: Option<Box<dyn ValidateParsedValue<String>>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalAnyText(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part to parse an entity name.
pub fn entity_part(id: CommandPartId<Entity>) -> CommandFormatPart {
    build_entity_part(id, None)
}

/// Creates a part to parse an entity name, with a validator function.
pub fn entity_part_with_validator(
    id: CommandPartId<Entity>,
    validator: Box<dyn ValidateParsedValue<Entity>>,
) -> CommandFormatPart {
    build_entity_part(id, Some(validator))
}

fn build_entity_part(
    id: CommandPartId<Entity>,
    validator: Option<Box<dyn ValidateParsedValue<Entity>>>,
) -> CommandFormatPart {
    CommandFormatPart::Entity(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part to parse an optional entity name.
pub fn optional_entity_part(id: CommandPartId<Option<Entity>>) -> CommandFormatPart {
    build_optional_entity_part(id, None)
}

/// Creates a part to parse an optional entity name, with a validator function.
pub fn optional_entity_part_with_validator(
    id: CommandPartId<Option<Entity>>,
    validator: Box<dyn ValidateParsedValue<Entity>>,
) -> CommandFormatPart {
    build_optional_entity_part(id, Some(validator))
}

fn build_optional_entity_part(
    id: CommandPartId<Option<Entity>>,
    validator: Option<Box<dyn ValidateParsedValue<Entity>>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalEntity(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part to parse a direction.
pub fn direction_part(id: CommandPartId<Direction>) -> CommandFormatPart {
    build_direction_part(id, None)
}

/// Creates a part to parse a direction, with a validator function.
pub fn direction_part_with_validator(
    id: CommandPartId<Direction>,
    validator: Box<dyn ValidateParsedValue<Direction>>,
) -> CommandFormatPart {
    build_direction_part(id, Some(validator))
}

fn build_direction_part(
    id: CommandPartId<Direction>,
    validator: Option<Box<dyn ValidateParsedValue<Direction>>>,
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
    validator: Box<dyn ValidateParsedValue<Direction>>,
) -> CommandFormatPart {
    build_optional_direction_part(id, Some(validator))
}

fn build_optional_direction_part(
    id: CommandPartId<Option<Direction>>,
    validator: Option<Box<dyn ValidateParsedValue<Direction>>>,
) -> CommandFormatPart {
    CommandFormatPart::OptionalDirection(CommandFormatPartParams {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        validator,
    })
}

/// Creates a part that consumes one of a set of possible things.
/// Inherits the options from the first part in the provided list.
pub fn one_of_part(parts: NonEmpty<CommandFormatPart>) -> CommandFormatPart {
    let options = parts.first().options().clone();
    CommandFormatPart::OneOf(parts.into_iter().collect(), options)
}

/// An identifier for a part of a command to be used to retrieve the parsed value.
/// `T` is the type that the part will be parsed into.
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
        for id in &part.all_ids() {
            if self
                .0
                .iter()
                .any(|existing_part| existing_part.all_ids().contains(id))
            {
                panic!("Duplicate command part ID: {}", id.0)
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

/// An error encountered while parsing input into a command.
/// TODO rename
#[derive(Debug)]
pub enum CommandParseErrorNew {
    /// An error occurred when attempting to parse a part
    Part {
        matched_parts: Vec<MatchedCommandFormatPart>,
        // boxed to reduce size
        unmatched_part: Box<CommandFormatPart>,
        error: CommandPartParseError,
    },
    /// Some of the input remained unmatched after all the parsers were run
    UnmatchedInput {
        matched_parts: Vec<MatchedCommandFormatPart>,
        unmatched: String,
    },
}

impl CommandParseErrorNew {
    /// Returns true if at least one part was matched, false if no parts were matched.
    pub fn any_parts_matched(&self) -> bool {
        let matched_parts = match self {
            CommandParseErrorNew::Part { matched_parts, .. } => matched_parts,
            CommandParseErrorNew::UnmatchedInput { matched_parts, .. } => matched_parts,
        };

        !matched_parts.is_empty()
    }

    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self, context: PartParserContext, world: &World) -> GameMessage {
        if !self.any_parts_matched() {
            return GameMessage::Error("I don't understand that.".to_string());
        }

        let string = match self {
            CommandParseErrorNew::Part {
                matched_parts,
                unmatched_part,
                error,
            } => {
                let matched_parts_string = matched_parts
                    .into_iter()
                    .map(|matched_part| {
                        matched_part.to_string_for_parse_error(context.clone(), world)
                    })
                    .join("");

                let error_detail_string = match error {
                    CommandPartParseError::EndOfInput => None,
                    CommandPartParseError::Unmatched { details } => details,
                    CommandPartParseError::Invalid(error) => error.details,
                }
                .map(|message| format!(" ({message})"))
                .unwrap_or_default();

                let unmatched_part_string =
                    unmatched_part.options().if_missing.as_deref().unwrap_or("");

                format!("{matched_parts_string}{unmatched_part_string}?{error_detail_string}")
            }
            CommandParseErrorNew::UnmatchedInput {
                matched_parts,
                unmatched,
            } => {
                let matched_parts_string = matched_parts
                    .into_iter()
                    .map(|matched_part| {
                        matched_part.to_string_for_parse_error(context.clone(), world)
                    })
                    .join("");

                format!("Did you mean '{matched_parts_string}' (without '{unmatched}')?")
            }
        };

        GameMessage::Error(string)
    }
}

//TODO give this a better name
pub enum ProcessedCommandFormatPart {
    Matched(MatchedCommandFormatPart),
    Unmatched(CommandFormatPart),
}

#[derive(Debug)]
pub struct MatchedCommandFormatPart {
    part: CommandFormatPart,
    matched_input: String,
    parsed_value: ParsedValue,
}

impl MatchedCommandFormatPart {
    /// Builds a string representing this part to use in a parsing error message.
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        let options = self.part.options();
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
    parsed_parts: HashMap<UntypedCommandPartId, MatchedCommandFormatPart>,
}

impl ParsedCommand {
    /// Creates a `ParsedCommand` from a list of matched parts.
    fn new(all_parsed_parts: Vec<MatchedCommandFormatPart>) -> ParsedCommand {
        let mut parsed_parts = HashMap::new();
        for parsed_part in all_parsed_parts {
            if let Some(id) = parsed_part.part.id() {
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
        let parsed_value = self
            .parsed_parts
            .get(&UntypedCommandPartId(id.0.clone()))
            .map(|matched_part| &matched_part.parsed_value)
            .unwrap_or_else(|| panic!("No part found for ID {}", id.0));

        parsed_value.clone().try_into().unwrap_or_else(|_| {
            panic!(
                "Unable to convert {:?} to {}",
                parsed_value,
                type_name::<T>()
            )
        })
    }
}

impl CommandFormat {
    /// Attempts to parse the provided input with this format.
    pub fn parse(
        &self,
        input: impl Into<String>,
        entering_entity: Entity,
        world: &World,
    ) -> Result<ParsedCommand, CommandParseErrorNew> {
        let mut remaining_input = input.into();
        let mut has_remaining_input = true;
        let mut parsed_parts = Vec::new();
        for (i, part) in self.0.iter().enumerate() {
            if remaining_input.is_empty() {
                has_remaining_input = false;
            }

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

                    parsed_parts.push(MatchedCommandFormatPart {
                        part: part.clone(),
                        matched_input: consumed,
                        parsed_value: parsed,
                    });

                    remaining_input = remaining;
                }
                CommandPartParseResult::Failure { error, .. } => {
                    if !has_remaining_input {
                        // Assume that this part failed to parse due to the input being empty. This has to be down here because some parts
                        // may be optional, in which case they will parse just fine with no input, so this shouldn't pre-emptively return
                        // an end of input error without letting the part see if that's actually a problem first.
                        //TODO is it a problem to just throw away the error returned from the part?
                        return Err(CommandParseErrorNew::Part {
                            matched_parts: parsed_parts,
                            unmatched_part: Box::new(part.clone()),
                            error: CommandPartParseError::EndOfInput,
                        });
                    }

                    return Err(CommandParseErrorNew::Part {
                        matched_parts: parsed_parts,
                        unmatched_part: Box::new(part.clone()),
                        error,
                    });
                }
            }
        }

        if !remaining_input.is_empty() {
            return Err(CommandParseErrorNew::UnmatchedInput {
                matched_parts: parsed_parts,
                unmatched: remaining_input,
            });
        }

        Ok(ParsedCommand::new(parsed_parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonempty::nonempty;

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
                (Self::AnyText(l0), Self::AnyText(r0)) => l0 == r0,
                (Self::OptionalAnyText(l0), Self::OptionalAnyText(r0)) => l0 == r0,
                (Self::Entity(l0), Self::Entity(r0)) => l0 == r0,
                (Self::OptionalEntity(l0), Self::OptionalEntity(r0)) => l0 == r0,
                (Self::Direction(l0), Self::Direction(r0)) => l0 == r0,
                (Self::OptionalDirection(l0), Self::OptionalDirection(r0)) => l0 == r0,
                (Self::OneOf(l0, l1), Self::OneOf(r0, r1)) => l0 == r0 && l1 == r1,
                _ => false,
            }
        }
    }

    impl<P: PartialEq, V> PartialEq for CommandFormatPartParams<P, V> {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.options == other.options
        }
    }

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

    #[test]
    fn format() {
        let format = CommandFormat::new(literal_part("first part"))
            .then(entity_part(CommandPartId::new("entityPartId")).with_if_missing("what"))
            .then(literal_part("third part"))
            .then(any_text_part(CommandPartId::new("anyTextPartId")))
            .then(optional_literal_part("optional part"))
            .then(one_of_part(nonempty![
                literal_part("option 1"),
                literal_part("option 2")
            ]));

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
                    },
                    validator: None,
                }
            ),
            CommandFormatPart::Entity(CommandFormatPartParams {
                id: Some(CommandPartId::new("entityPartId")),
                options: CommandFormatPartOptions {
                    if_missing: Some("what".to_string()),
                    format_description_part_type: CommandFormatDescriptionPartType::Nothing,
                    include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                    error_string_override: None,
                },
                validator: None,
            }),
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
                    },
                    validator: None,
                }
            ),
            CommandFormatPart::OneOf(
                vec![
                    CommandFormatPart::Literal(
                        "option 1".to_string(),
                        CommandFormatPartParams {
                            id: None,
                            options: CommandFormatPartOptions {
                                if_missing: None,
                                format_description_part_type:
                                    CommandFormatDescriptionPartType::Literal(
                                        "option 1".to_string()
                                    ),
                                include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                                error_string_override: None,
                            },
                            validator: None,
                        }
                    ),
                    CommandFormatPart::Literal(
                        "option 2".to_string(),
                        CommandFormatPartParams {
                            id: None,
                            options: CommandFormatPartOptions {
                                if_missing: None,
                                format_description_part_type:
                                    CommandFormatDescriptionPartType::Literal(
                                        "option 2".to_string()
                                    ),
                                include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                                error_string_override: None,
                            },
                            validator: None,
                        }
                    )
                ],
                CommandFormatPartOptions {
                    if_missing: None,
                    format_description_part_type: CommandFormatDescriptionPartType::Literal(
                        "option 1".to_string()
                    ),
                    include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                    error_string_override: None,
                },
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
            .then(any_text_part(CommandPartId::new("anyTextPartId")))
            .then(one_of_part(nonempty![
                literal_part("some literal"),
                entity_part(CommandPartId::new("somePartId")),
            ]));
    }

    #[test]
    fn format_nested_one_of() {
        let format = CommandFormat::new(literal_part("first part")).then(one_of_part(nonempty![
            literal_part("option 1"),
            one_of_part(nonempty![
                literal_part("option 2.1"),
                literal_part("option 2.2"),
            ]),
        ]));

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
                    },
                    validator: None,
                }
            ),
            CommandFormatPart::OneOf(
                vec![
                    CommandFormatPart::Literal(
                        "option 1".to_string(),
                        CommandFormatPartParams {
                            id: None,
                            options: CommandFormatPartOptions {
                                if_missing: None,
                                format_description_part_type:
                                    CommandFormatDescriptionPartType::Literal(
                                        "option 1".to_string()
                                    ),
                                include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                                error_string_override: None
                            },
                            validator: None
                        }
                    ),
                    CommandFormatPart::OneOf(
                        vec![
                            CommandFormatPart::Literal(
                                "option 2.1".to_string(),
                                CommandFormatPartParams {
                                    id: None,
                                    options: CommandFormatPartOptions {
                                        if_missing: None,
                                        format_description_part_type:
                                            CommandFormatDescriptionPartType::Literal(
                                                "option 2.1".to_string()
                                            ),
                                        include_in_errors_behavior:
                                            IncludeInErrorsBehavior::OnlyIfMatched,
                                        error_string_override: None
                                    },
                                    validator: None
                                }
                            ),
                            CommandFormatPart::Literal(
                                "option 2.2".to_string(),
                                CommandFormatPartParams {
                                    id: None,
                                    options: CommandFormatPartOptions {
                                        if_missing: None,
                                        format_description_part_type:
                                            CommandFormatDescriptionPartType::Literal(
                                                "option 2.2".to_string()
                                            ),
                                        include_in_errors_behavior:
                                            IncludeInErrorsBehavior::OnlyIfMatched,
                                        error_string_override: None
                                    },
                                    validator: None
                                }
                            ),
                        ],
                        CommandFormatPartOptions {
                            if_missing: None,
                            format_description_part_type: CommandFormatDescriptionPartType::Literal(
                                "option 2.1".to_string()
                            ),
                            include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                            error_string_override: None,
                        }
                    ),
                ],
                CommandFormatPartOptions {
                    if_missing: None,
                    format_description_part_type: CommandFormatDescriptionPartType::Literal(
                        "option 1".to_string()
                    ),
                    include_in_errors_behavior: IncludeInErrorsBehavior::OnlyIfMatched,
                    error_string_override: None
                },
            ),
        ]);

        assert_eq!(expected, format);
    }
}
