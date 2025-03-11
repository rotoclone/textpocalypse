use itertools::Itertools;
use std::{
    any::{type_name, Any},
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
};
use voca_rs::Voca;

use bevy_ecs::prelude::*;

use nonempty::{nonempty, NonEmpty};

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

//TODO add a variant for some kind of "sub-list" of parts, so you can do stuff like define optionally having a whole sequence of parts as whole, rather than having to make them optional individually
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
    /// TODO is that what should happen for `OneOf` parts?
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

    /// Sets the string to include in the error message if this part is missing (e.g. "what", "who", etc.).
    pub fn with_if_missing(mut self, s: impl Into<String>) -> Self {
        self.options_mut().if_missing = Some(s.into());
        self
    }

    /// Sets the literal string to include in the command's format string for this part (e.g. "get", "look", etc.).
    pub fn with_literal_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options_mut().format_string_part_type =
            CommandFormatStringPartType::Literal(name.into());
        self
    }

    /// Sets the name of the placeholder to include in the command's format string for this part (e.g. "thing", "target", etc.).
    pub fn with_placeholder_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options_mut().format_string_part_type =
            CommandFormatStringPartType::Placeholder(name.into());
        self
    }

    /// Sets the part to always be included in error messages, regardless of if it was included in the entered command.
    pub fn always_include_in_errors(mut self) -> Self {
        self.options_mut().include_in_errors_behavior = IncludeInErrorsBehavior::Always;
        self
    }

    /// Sets the part to never be included in error messages, regardless of if it was included in the entered command.
    pub fn never_include_in_errors(mut self) -> Self {
        self.options_mut().include_in_errors_behavior = IncludeInErrorsBehavior::Never;
        self
    }

    /// TODO doc
    pub fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<ParsedValue> {
        //TODO call validators at some point
        match self {
            CommandFormatPart::Literal(literal, _) => parse_literal(literal, context),
            CommandFormatPart::OptionalLiteral(literal, _) => {
                parse_result_to_option(parse_literal(literal, context))
            }
            CommandFormatPart::AnyText(_) => parse_any_text(context),
            CommandFormatPart::OptionalAnyText(_) => {
                parse_result_to_option(parse_any_text(context))
            }
            CommandFormatPart::Entity(_) => parse_entity(context, world),
            CommandFormatPart::OptionalEntity(_) => {
                parse_result_to_option(parse_entity(context, world))
            }
            CommandFormatPart::Direction(_) => parse_direction(context),
            CommandFormatPart::OptionalDirection(_) => {
                parse_result_to_option(parse_direction(context))
            }
            CommandFormatPart::OneOf(parts, _) => parse_one_of(parts, context, world),
        }
    }
}

/// Parses a literal value from the provided context.
fn parse_literal(literal: &str, context: PartParserContext) -> CommandPartParseResult<ParsedValue> {
    if let Some(remaining) = context.input.strip_prefix(literal) {
        return CommandPartParseResult::Success {
            parsed: ParsedValue::String(literal.to_string()),
            consumed: literal.to_string(),
            remaining: remaining.to_string(),
        };
    }

    CommandPartParseResult::Failure {
        error: CommandPartParseError::Unmatched,
        remaining: context.input,
    }
}

/// Parses all the text from the provided context.
/// If the next part to be parsed is a literal, this will stop once that literal is reached.
fn parse_any_text(context: PartParserContext) -> CommandPartParseResult<ParsedValue> {
    let (parsed, remaining) = take_until_literal_if_next(context);

    CommandPartParseResult::Success {
        parsed: ParsedValue::String(parsed.clone()),
        consumed: parsed,
        remaining: remaining.to_string(),
    }
}

/// Parses an entity from the provided context.
fn parse_entity(context: PartParserContext, world: &World) -> CommandPartParseResult<ParsedValue> {
    EntityParser.parse(context, world).into_generic()
}

/// Parses a direction from the provided context.
fn parse_direction(context: PartParserContext) -> CommandPartParseResult<ParsedValue> {
    let (to_parse, remaining) = take_until_literal_if_next(context);

    if let Some(direction) = Direction::parse(&to_parse) {
        CommandPartParseResult::Success {
            parsed: ParsedValue::Direction(direction),
            consumed: to_parse,
            remaining,
        }
    } else {
        CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched,
            // re-combine here to avoid an extra clone above in non-error cases
            remaining: format!("{to_parse}{remaining}"),
        }
    }
}

/// Finds the first part in `parts` that successfully parses.
/// If none of the parts parse successfully, returns the first error encountered.
fn parse_one_of(
    parts: &[CommandFormatPart],
    context: PartParserContext,
    world: &World,
) -> CommandPartParseResult<ParsedValue> {
    let mut first_error = None;
    for part in parts {
        match part.parse(context.clone(), world) {
            CommandPartParseResult::Success {
                parsed,
                consumed,
                remaining,
            } => {
                return CommandPartParseResult::Success {
                    parsed,
                    consumed,
                    remaining,
                };
            }
            CommandPartParseResult::Failure { error, .. } => {
                first_error.get_or_insert(error);
            }
        }
    }

    CommandPartParseResult::Failure {
        error: first_error.unwrap_or(CommandPartParseError::Unmatched),
        remaining: context.input,
    }
}

/// If the next part is a literal: returns a tuple of the input up until the literal, and the input including and after the literal.
///
/// If the next part is not a literal: returns `(input, "")`.
fn take_until_literal_if_next(context: PartParserContext) -> (String, String) {
    let stopping_point = if let Some(CommandFormatPart::Literal(literal, _)) = context.next_part {
        Some(literal)
    } else {
        None
    };

    take_until(context.input, stopping_point)
}

/// Splits `input` at the first instance of `stopping_point`, returning a tuple of the input before `stopping_point`, and the input including and after `stopping_point`.
/// If `stopping_point` is `None`, returns `(input, "")`.
fn take_until(input: impl Into<String>, stopping_point: Option<&String>) -> (String, String) {
    let input = input.into();
    if let Some(stopping_point) = stopping_point {
        let parsed = input._before(stopping_point);
        let remaining = input.strip_prefix(&parsed).unwrap_or_default();
        (parsed, remaining.to_string())
    } else {
        (input.clone(), "".to_string())
    }
}

/// Converts `CommandPartParseResult::Success` to have a parsed value of `Option(...)`, and `CommandPartParseResult::Failure` to `CommandPartParseResult::Success` with a parsed value of `Option(None)`
fn parse_result_to_option(
    parse_result: CommandPartParseResult<ParsedValue>,
) -> CommandPartParseResult<ParsedValue> {
    match parse_result {
        CommandPartParseResult::Success {
            parsed,
            consumed,
            remaining,
        } => CommandPartParseResult::Success {
            parsed: ParsedValue::Option(Some(Box::new(parsed))),
            consumed,
            remaining,
        },
        CommandPartParseResult::Failure {
            error: _,
            remaining,
        } => CommandPartParseResult::Success {
            parsed: ParsedValue::Option(None),
            consumed: "".to_string(),
            remaining,
        },
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

//TODO probably remove these
impl<P, V> CommandFormatPartParams<P, V> {
    /// Adds a validator to this part. Any existing validator will be replaced.
    pub fn with_validator(mut self, validator: Box<dyn ValidateParsedValue<V>>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Sets the string to include in the error message if this part is missing (e.g. "what", "who", etc.).
    pub fn with_if_missing(mut self, s: impl Into<String>) -> Self {
        self.options.if_missing = Some(s.into());
        self
    }

    /// Sets the literal string to include in the command's format string for this part (e.g. "get", "look", etc.).
    pub fn with_literal_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options.format_string_part_type = CommandFormatStringPartType::Literal(name.into());
        self
    }

    /// Sets the name of the placeholder to include in the command's format string for this part (e.g. "thing", "target", etc.).
    pub fn with_placeholder_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options.format_string_part_type =
            CommandFormatStringPartType::Placeholder(name.into());
        self
    }

    /// Sets the part to always be included in error messages, regardless of if it was included in the entered command.
    pub fn always_include_in_errors(mut self) -> Self {
        self.options.include_in_errors_behavior = IncludeInErrorsBehavior::Always;
        self
    }

    /// Sets the part to never be included in error messages, regardless of if it was included in the entered command.
    pub fn never_include_in_errors(mut self) -> Self {
        self.options.include_in_errors_behavior = IncludeInErrorsBehavior::Never;
        self
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct CommandFormatPartOptions {
    /// The string to include in the error message if this part is missing (e.g. "what", "who", etc.)
    if_missing: Option<String>,
    /// The string to include in the command's format string for this part (e.g. "thing", "target", etc.).
    /// If `None`, the part will not be included in the format string.
    format_string_part_type: CommandFormatStringPartType,
    /// When to include this part in error messages.
    include_in_errors_behavior: IncludeInErrorsBehavior,
}

/// Specifies when to include a part in an error message.
/// TODO does this even make sense? Won't a part always be matched unless it's wrapped in a "maybe" part?
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum IncludeInErrorsBehavior {
    /// The part is always included in error messages, even if it was not included in the entered command.
    Always,
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

/// Creates a part to consume a literal value, with a validator function.
/// TODO but it doesn't make any sense to have a custom validator, it'll always validate the literal value...unless the validation depends on the world state? is that a valid use case?
pub fn literal_part_with_validator(
    literal: impl Into<String>,
    validator: Box<dyn ValidateParsedValue<String>>,
) -> CommandFormatPart {
    build_literal_part(literal, Some(validator))
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
                format_string_part_type: CommandFormatStringPartType::Literal(literal_string),
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

/// Creates a part to maybe consume a literal value, with a validator function.
/// TODO but it doesn't make any sense to have a custom validator, it'll always validate the literal value...unless the validation depends on the world state? is that a valid use case?
pub fn optional_literal_part_with_validator(
    literal: impl Into<String>,
    validator: Box<dyn ValidateParsedValue<String>>,
) -> CommandFormatPart {
    build_optional_literal_part(literal, Some(validator))
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
                format_string_part_type: CommandFormatStringPartType::Literal(literal_string),
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
        Self(self.0.clone(), self.1.clone())
    }
}

impl<T> CommandPartId<T> {
    /// Creates a new part ID.
    pub fn new(value: impl Into<String>) -> CommandPartId<T> {
        CommandPartId(value.into(), PhantomData)
    }
}

//TODO add some kind of function for detecting if an input starts with the verb for a format, for example to differentiate between an invalid look, and invalid examine, or just a different command
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
                panic!("Duplicate command part ID: {id:?}")
            }
        }

        self.0.push(part);
    }

    /// Gets the format string for this command format, to demonstrate how it should be used.
    /// TODO rename this since it doesn't actually return a string
    pub fn get_format_string(&self) -> CommandFormatString {
        CommandFormatString::new(
            self.0
                .iter()
                .map(|part| CommandFormatStringPart {
                    id: part.id().clone(),
                    part_type: part.options().format_string_part_type.clone(),
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
        // TODO try without box now?
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
    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self, context: PartParserContext, world: &World) -> GameMessage {
        let string = match self {
            CommandParseErrorNew::Part {
                matched_parts,
                unmatched_part,
                error,
            } => {
                if matched_parts.is_empty() {
                    //TODO special message
                }
                //TODO take into account options
                let matched_parts_string = matched_parts
                    .into_iter()
                    .map(|matched_part| {
                        matched_part
                            .parsed_value
                            .to_string_for_parse_error(context.clone(), world)
                    })
                    .join("");

                let error_detail_string = match error {
                    CommandPartParseError::EndOfInput => "",
                    CommandPartParseError::Unmatched => todo!(),
                    CommandPartParseError::Invalid(command_part_validate_error) => todo!(),
                };

                let unmatched_part_string = unmatched_part
                    .options()
                    .if_missing
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("");

                format!("{matched_parts_string}{unmatched_part_string}?{error_detail_string}")
            }
            CommandParseErrorNew::UnmatchedInput {
                matched_parts,
                unmatched,
            } => todo!(),
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

pub struct ParsedCommand {
    parsed_parts: HashMap<UntypedCommandPartId, MatchedCommandFormatPart>,
}

impl ParsedCommand {
    /// Gets the parsed value associated with `id`.
    /// Panics if the ID does not correspond to a part on this command, or the parsed value for this ID isn't a `T`.
    pub fn get<T: 'static>(&self, id: &CommandPartId<T>) -> T
    where
        ParsedValue: TryInto<T>,
    {
        let parsed_value = self
            .parsed_parts
            //TODO remove this clone if possible
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
        let mut parsed_parts = HashMap::new();
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

                    if let Some(id) = &part.id() {
                        parsed_parts.insert(
                            id.clone(),
                            MatchedCommandFormatPart {
                                part: part.clone(),
                                matched_input: consumed,
                                parsed_value: parsed,
                            },
                        );
                    }

                    remaining_input = remaining;
                }
                CommandPartParseResult::Failure { error, .. } => {
                    if !has_remaining_input {
                        // Assume that this part failed to parse due to the input being empty. This has to be down here because some parts
                        // may be optional, in which case they will parse just fine with no input, so this shouldn't pre-emptively return
                        // an end of input error without letting the part see if that's actually a problem first.
                        //TODO is it a problem to just throw away the error returned from the part?
                        return Err(CommandParseErrorNew::Part {
                            matched_parts: parsed_parts.into_values().collect(),
                            unmatched_part: Box::new(part.clone()),
                            error: CommandPartParseError::EndOfInput,
                        });
                    }

                    return Err(CommandParseErrorNew::Part {
                        matched_parts: parsed_parts.into_values().collect(),
                        unmatched_part: Box::new(part.clone()),
                        error,
                    });
                }
            }
        }

        if !remaining_input.is_empty() {
            return Err(CommandParseErrorNew::UnmatchedInput {
                matched_parts: parsed_parts.into_values().collect(),
                unmatched: remaining_input,
            });
        }

        Ok(ParsedCommand { parsed_parts })
    }
}

/* TODO
#[cfg(test)]
mod tests {
    use super::*;

    fn entity_validator_fn(_: CommandPartParseContext<Entity>, _: &World) -> bool {
        true
    }

    #[test]
    fn format() {
        let format = CommandFormat::new_with_literal("first part")
            .then_entity(CommandPartId::new("entityPartId"), "what", None)
            .then_literal("third part")
            .then_any_text(CommandPartId::new("anyTextPartId"))
            .then_maybe(
                CommandPartId::new("optionalPartId"),
                literal_part("optional part"),
            )
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![literal_part("option 1"), literal_part("option 2")],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(TypedCommandPartId::Entity(CommandPartId::new(
                    "entityPartId"
                ))),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: None,
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(TypedCommandPartId::AnyText(CommandPartId::new(
                    "anyTextPartId"
                ))),
                CommandFormatPart::AnyText
            ),
            (
                Some(TypedCommandPartId::Maybe(Box::new(
                    TypedCommandPartId::Literal(CommandPartId::new("optionalPartId"))
                ))),
                CommandFormatPart::Maybe(Box::new(CommandFormatPart::Literal(
                    "optional part".to_string()
                )))
            ),
            (
                Some(TypedCommandPartId::OneOf(CommandPartId::new("oneOfPartId"))),
                CommandFormatPart::OneOf(Box::new(nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::Literal("option 2".to_string())
                ])),
            ),
        ]);

        assert_eq!(expected, format);
    }

    #[test]
    fn format_with_entity_validator_fn() {
        let format = CommandFormat::new_with_literal("first part")
            .then_entity(
                CommandPartId::new("entityPartId"),
                "what",
                Some(entity_validator_fn),
            )
            .then_literal("third part")
            .then_any_text(CommandPartId::new("anyTextPartId"))
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![literal_part("option 1"), literal_part("option 2")],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(TypedCommandPartId::Entity(CommandPartId::new(
                    "entityPartId"
                ))),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: Some(entity_validator_fn),
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(TypedCommandPartId::AnyText(CommandPartId::new(
                    "anyTextPartId"
                ))),
                CommandFormatPart::AnyText
            ),
            (
                Some(TypedCommandPartId::OneOf(CommandPartId::new("oneOfPartId"))),
                CommandFormatPart::OneOf(Box::new(nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::Literal("option 2".to_string())
                ])),
            ),
        ]);

        assert_eq!(expected, format);
    }

    #[test]
    #[should_panic = "Duplicate command part ID: somePartId"]
    fn format_duplicate_ids() {
        CommandFormat::new_with_literal("first part")
            .then_entity(CommandPartId::new("somePartId"), "what", None)
            .then_literal("third part")
            .then_any_text(CommandPartId::new("anyTextPartId"))
            .then_one_of(
                CommandPartId::new("somePartId"),
                nonempty![literal_part("option 1"), literal_part("option 2")],
            );
    }

    #[test]
    fn format_nested_one_of() {
        let format = CommandFormat::new_with_literal("first part")
            .then_entity(CommandPartId::new("entityPartId"), "what", None)
            .then_literal("third part")
            .then_any_text(CommandPartId::new("anyTextPartId"))
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![
                    literal_part("option 1"),
                    one_of_part(nonempty![
                        literal_part("option 2.1"),
                        literal_part("option 2.2")
                    ]),
                ],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(TypedCommandPartId::Entity(CommandPartId::new(
                    "entityPartId"
                ))),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: None,
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(TypedCommandPartId::AnyText(CommandPartId::new(
                    "anyTextPartId"
                ))),
                CommandFormatPart::AnyText
            ),
            (
                Some(TypedCommandPartId::OneOf(CommandPartId::new("oneOfPartId"))),
                CommandFormatPart::OneOf(Box::new(nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::OneOf(Box::new(nonempty![
                        CommandFormatPart::Literal("option 2.1".to_string()),
                        CommandFormatPart::Literal("option 2.2".to_string())
                    ])),
                ])),
            ),
        ]);

        assert_eq!(expected, format);
    }
}
*/
