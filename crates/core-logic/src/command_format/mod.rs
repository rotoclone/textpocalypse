use std::{
    any::{type_name, Any},
    collections::HashMap,
    fmt::Display,
    marker::PhantomData,
    ops::Deref,
};

use bevy_ecs::prelude::*;

mod part_parsers;
use itertools::Itertools;
use part_parsers::*;

mod parsed_value_validators;
use parsed_value_validators::*;

use nonempty::{nonempty, NonEmpty};

use crate::{Description, GameMessage};

/// The format of a command a player can enter.
/// TODO change to a regular Vec instead of NonEmpty?
#[derive(Debug)]
pub struct CommandFormat(NonEmpty<UntypedCommandFormatPart>);

/// A `CommandPartId` with no associated type information, so different ones can be put in a collection together.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct UntypedCommandPartId(String);

impl<T> From<CommandPartId<T>> for UntypedCommandPartId {
    fn from(val: CommandPartId<T>) -> Self {
        UntypedCommandPartId(val.0)
    }
}

/// A `CommandFormatPart` with no associated type information, so different ones can be put in a collection together.
#[derive(Debug)]
pub struct UntypedCommandFormatPart {
    id: Option<UntypedCommandPartId>,
    options: CommandFormatPartOptions,
    parser: Box<dyn ParsePartUntyped>,
    validator: Option<Box<dyn ValidateParsedValueUntyped>>,
}

impl Clone for UntypedCommandFormatPart {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            options: self.options.clone(),
            parser: self.parser.clone_box(),
            validator: self.validator.as_ref().map(|v| v.clone_box()),
        }
    }
}

impl<T> From<CommandFormatPart<T>> for UntypedCommandFormatPart {
    fn from(val: CommandFormatPart<T>) -> Self {
        UntypedCommandFormatPart {
            id: val.id.map(|id| id.into()),
            options: val.options,
            parser: val.parser.as_untyped(),
            validator: val.validator.map(|v| v.as_untyped()),
        }
    }
}

/* TODO remove
impl<T: ValidateParsedValue<P>, P: 'static> ValidateParsedValueUntyped for T {
    fn validate(
        &self,
        context: PartValidatorContext<Box<dyn Any>>,
        world: &World,
    ) -> CommandPartValidateResult {
        self.validate(
            PartValidatorContext {
                parsed_value: *context
                    .parsed_value
                    .downcast::<P>()
                    .expect("parsed value type should match"),
                performing_entity: context.performing_entity,
            },
            world,
        )
    }
}
    */

#[derive(Debug)]
pub struct CommandFormatPart<T> {
    id: Option<CommandPartId<T>>,
    options: CommandFormatPartOptions,
    parser: Box<dyn ParsePart<T>>,
    validator: Option<Box<dyn ValidateParsedValue<T>>>,
}

impl<T: Clone> Clone for CommandFormatPart<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            options: self.options.clone(),
            parser: ParsePartClone::clone_box(self.parser.deref()),
            validator: self
                .validator
                .as_ref()
                .map(|v| ValidateParsedValueClone::clone_box(v.deref())),
        }
    }
}

impl<T> CommandFormatPart<T> {
    /// Adds a validator to this part. Any existing validator will be replaced.
    pub fn with_validator(mut self, validator: Box<dyn ValidateParsedValue<T>>) -> Self {
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

/// Describes what to include in the format string for a part.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
enum CommandFormatStringPartType {
    /// This part shouldn't be included in the format string.
    #[default]
    Nothing,
    /// A literal string, for parts of the format that must be entered literally to be matched (e.g. "get", "look", etc.)
    Literal(String),
    /// A placeholder for a target, (e.g. "thing", "target", etc.)
    Placeholder(String),
}

/// Specifies when to include a part in an error message.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum IncludeInErrorsBehavior {
    /// The part is always included in error messages, even if it was not included in the entered command.
    Always,
    /// The part is never included in error messages, even if it was included in the entered command.
    Never,
    /// The part is only included in an error message if it was in the entered command.
    #[default]
    OnlyIfMatched,
}

struct CommandFormatStringPart {
    id: Option<UntypedCommandPartId>,
    part_type: CommandFormatStringPartType,
}

pub struct CommandFormatString {
    parts: Vec<CommandFormatStringPart>,
    filled_placeholders: HashMap<UntypedCommandPartId, String>,
}

impl CommandFormatString {
    /// Creates a new format string with no placeholders filled in.
    fn new(parts: Vec<CommandFormatStringPart>) -> CommandFormatString {
        CommandFormatString {
            parts,
            filled_placeholders: HashMap::new(),
        }
    }

    /// Fills the placeholder for the part with the provided ID with the name of the provided entity.
    /// Does nothing if the entity has no name.
    pub fn with_targeted_entity(
        mut self,
        id: CommandPartId<Entity>,
        entity: Entity,
        world: &World,
    ) -> Self {
        if let Some(name) = Description::get_name(entity, world) {
            self.filled_placeholders.insert(id.into(), name);
        }

        self
    }
}

impl Display for CommandFormatString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = self
            .parts
            .iter()
            .filter_map(|part| match &part.part_type {
                CommandFormatStringPartType::Nothing => None,
                CommandFormatStringPartType::Literal(l) => Some(l.to_string()),
                CommandFormatStringPartType::Placeholder(p) => Some(
                    part.id
                        .as_ref()
                        .and_then(|id| self.filled_placeholders.get(id).cloned())
                        .unwrap_or_else(|| format!("<{p}>")),
                ),
            })
            .join("");

        string.fmt(f)
    }
}

/* TODO remove
/// A piece of a command format.
/// TODO allow specifying which parts to include in an error message (for example, the "at" and "the" are optional in a "look" command, but if someone enters just "l" the error should probably be "look at what?" and not "l what?" or "look at the what?")
/// TODO allow providing names for parts so format strings can be generated (e.g. "look at <thing>")
/// TODO allow parts to be parsed into custom types (e.g. a `Direction` for the "move" command)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandFormatPartOld {
    /// A literal value.
    Literal(String),
    /// Any number of any characters.
    AnyText,
    /// A string that identifies an entity.
    Entity {
        /// The string to include in the error message if this part is missing (e.g. "what", "who", etc.)
        if_missing: String,
        /// The function to use to check whether the chosen Entity is valid.
        /// If this returns `false`, parsing will fail.
        validator: Option<EntityPartValidatorFn>,
    },
    /// Maybe a part, or maybe nothing.
    Maybe(Box<CommandFormatPartOld>),
    /// One of the provided part types.
    /// Matching will be attempted in order.
    /// TODO allow some way to tell which one was matched after parsing
    OneOf(Box<NonEmpty<CommandFormatPartOld>>),
}
    */

/// Creates a part to consume a literal value.
pub fn literal_part(literal: impl Into<String>) -> CommandFormatPart<String> {
    let literal_string = literal.into();
    CommandFormatPart {
        id: None,
        options: CommandFormatPartOptions {
            format_string_part_type: CommandFormatStringPartType::Literal(literal_string.clone()),
            ..Default::default()
        },
        parser: Box::new(LiteralParser(literal_string)),
        validator: None,
    }
}

/// Creates a part to consume any text.
pub fn any_text_part(id: CommandPartId<String>) -> CommandFormatPart<String> {
    CommandFormatPart {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        parser: Box::new(AnyTextParser),
        validator: None,
    }
}

/// Creates an `Entity` part.
pub fn entity_part(id: CommandPartId<Entity>) -> CommandFormatPart<Entity> {
    CommandFormatPart {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        parser: Box::new(EntityParser),
        validator: None,
    }
}

/// Creates a part to maybe consume something.
pub fn maybe_part<T: 'static + std::fmt::Debug + Clone>(
    id: CommandPartId<Option<T>>,
    //TODO this part doesn't need an associated ID
    part: CommandFormatPart<T>,
) -> CommandFormatPart<Option<T>> {
    CommandFormatPart {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        parser: Box::new(MaybeParser(part)),
        validator: None,
    }
}

/// Creates a part that consumes one of a set of possible things.
/// Inherits the options from the first part in the provided list.
pub fn one_of_part(parts: NonEmpty<UntypedCommandFormatPart>) -> CommandFormatPart<Box<dyn Any>> {
    CommandFormatPart {
        id: None,
        options: parts.first().options.clone(),
        parser: Box::new(OneOfParser(parts)),
        validator: None,
    }
}

/// An identifier for a part of a command to be used to retrieve the parsed value.
/// `T` is the type that the part will be parsed into.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPartId<T>(String, PhantomData<fn(T)>);

impl<T> CommandPartId<T> {
    /// Creates a new part ID.
    pub fn new(value: impl Into<String>) -> CommandPartId<T> {
        CommandPartId(value.into(), PhantomData)
    }
}

impl CommandFormat {
    /// Creates a format starting with the provided part.
    pub fn new<T: 'static + std::fmt::Debug>(part: CommandFormatPart<T>) -> CommandFormat {
        CommandFormat(NonEmpty::new(part.into()))
    }

    /// Adds a part to the format.
    /// Panics if the part has an ID and there is already a part with the same ID.
    pub fn then<T: 'static + std::fmt::Debug>(
        mut self,
        part: CommandFormatPart<T>,
    ) -> CommandFormat {
        self.add_part(part.into());
        self
    }

    /// Adds a part to the format.
    /// Panics if the part has an ID and there is already a part with the same ID.
    fn add_part(&mut self, part: UntypedCommandFormatPart) {
        if let Some(id) = &part.id {
            if self
                .0
                .iter()
                .any(|existing_part| existing_part.id.as_ref() == Some(id))
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
                    id: part.id.clone(),
                    part_type: part.options.format_string_part_type.clone(),
                })
                .collect(),
        )
    }
}

/// An error encountered while parsing input into a command.
/// TODO rename to `CommandParseError`
pub enum CommandFormatParseError {
    /// An error occurred when attempting to parse a part
    Part {
        matched_parts: Vec<UntypedCommandFormatPart>,
        error: CommandPartParseError,
    },
    /// Some of the input remained unmatched after all the parser were run
    UnmatchedInput {
        matched_parts: Vec<UntypedCommandFormatPart>,
        unmatched: String,
    },
}

impl CommandFormatParseError {
    /// Turns the error into a message to send to the entering entity describing what went wrong.
    pub fn into_message(self) -> GameMessage {
        todo!() //TODO
    }
}

pub struct ParsedCommand {
    parsed_parts: HashMap<UntypedCommandPartId, Box<dyn Any>>,
}

impl ParsedCommand {
    /// Gets the parsed value associated with `id`.
    /// Panics if the ID does not correspond to a part on this command.
    pub fn get<T: 'static>(&self, id: &CommandPartId<T>) -> &T {
        let parsed_value = self
            .parsed_parts
            //TODO remove this clone if possible
            .get(&UntypedCommandPartId(id.0.clone()))
            .unwrap_or_else(|| panic!("No part found for ID {}", id.0));

        parsed_value.downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unexpected parsed type for ID {} (expected {}): {:?}",
                id.0,
                type_name::<T>(),
                parsed_value
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
    ) -> Result<ParsedCommand, CommandFormatParseError> {
        let mut remaining_input = input.into();
        let mut matched_parts = Vec::new();
        let mut parsed_parts = HashMap::new();
        for part in &self.0 {
            match part.parser.parse_untyped(
                PartParserContext {
                    input: remaining_input,
                    entering_entity,
                },
                world,
            ) {
                CommandPartParseResult::Success { parsed, remaining } => {
                    matched_parts.push(part.clone());
                    if let Some(id) = &part.id {
                        parsed_parts.insert(id.clone(), parsed);
                    }

                    remaining_input = remaining;
                }
                CommandPartParseResult::Failure { error, .. } => {
                    return Err(CommandFormatParseError::Part {
                        matched_parts,
                        error,
                    })
                }
            }
        }

        if !remaining_input.is_empty() {
            return Err(CommandFormatParseError::UnmatchedInput {
                matched_parts,
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
