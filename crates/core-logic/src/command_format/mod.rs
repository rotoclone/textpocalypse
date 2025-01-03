use std::{
    any::{type_name, Any},
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
};

use bevy_ecs::prelude::*;

mod part_parsers;
use part_parsers::*;

mod parsed_value_validators;
use parsed_value_validators::*;

use nonempty::{nonempty, NonEmpty};

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

    /// Sets the string to include in the command's format string for this part (e.g. "thing", "target", etc.).
    pub fn with_name_for_format_string(mut self, name: impl Into<String>) -> Self {
        self.options.name_for_format_string = Some(name.into());
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
    name_for_format_string: Option<String>,
    /// When to include this part in error messages.
    include_in_errors_behavior: IncludeInErrorsBehavior,
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
    CommandFormatPart {
        id: None,
        options: CommandFormatPartOptions::default(),
        parser: Box::new(LiteralParser(literal.into())),
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
pub fn one_of_part(parts: NonEmpty<UntypedCommandFormatPart>) -> CommandFormatPart<Box<dyn Any>> {
    CommandFormatPart {
        id: None,
        options: CommandFormatPartOptions::default(),
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
}

/// An error encountered while parsing input into a command.
/// TODO rename to `CommandParseError`
pub enum CommandFormatParseError {
    /// A required part was not matched
    MissingPart,
    /// Something invalid was provided for a part
    InvalidPart,
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
    pub fn parse(&self, input: &str) -> Result<ParsedCommand, CommandFormatParseError> {
        todo!() //TODO
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
