use std::marker::PhantomData;

use bevy_ecs::prelude::*;

use nonempty::{nonempty, NonEmpty};

/// The format of a command a player can enter.
/// TODO change to a regular Vec instead of NonEmpty?
#[derive(Debug, PartialEq, Eq)]
pub struct CommandFormat(NonEmpty<(Option<TypedCommandPartId>, CommandFormatPart)>);

/// Enum of all the possible different generic `CommandPartId`s so different ones can be put in a collection together.
#[derive(Debug, PartialEq, Eq)]
enum TypedCommandPartId {
    AnyText(CommandPartId<AnyTextPartType>),
    Entity(CommandPartId<EntityPartType>),
    Maybe(CommandPartId<MaybePartType>),
    OneOf(CommandPartId<OneOfPartType>),
}

impl From<CommandPartId<AnyTextPartType>> for TypedCommandPartId {
    fn from(val: CommandPartId<AnyTextPartType>) -> Self {
        TypedCommandPartId::AnyText(val)
    }
}

impl From<CommandPartId<EntityPartType>> for TypedCommandPartId {
    fn from(val: CommandPartId<EntityPartType>) -> Self {
        TypedCommandPartId::Entity(val)
    }
}

impl From<CommandPartId<MaybePartType>> for TypedCommandPartId {
    fn from(val: CommandPartId<MaybePartType>) -> Self {
        TypedCommandPartId::Maybe(val)
    }
}

impl From<CommandPartId<OneOfPartType>> for TypedCommandPartId {
    fn from(val: CommandPartId<OneOfPartType>) -> Self {
        TypedCommandPartId::OneOf(val)
    }
}

pub type EntityPartValidatorFn = fn(CommandPartParseContext<Entity>, &World) -> bool;

/// A piece of a command format.
/// TODO allow specifying which parts to include in an error message (for example, the "at" and "the" are optional in a "look" command, but if someone enters just "l" the error should probably be "look at what?" and not "l what?" or "look at the what?")
/// TODO allow providing names for parts so format strings can be generated (e.g. "look at <thing>")
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandFormatPart {
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
    Maybe(Box<CommandFormatPart>),
    /// One of the provided part types.
    /// Matching will be attempted in order.
    /// TODO allow some way to tell which one was matched after parsing
    OneOf(Box<NonEmpty<CommandFormatPart>>),
}

/// Creates a `Literal` part.
pub fn literal_part(literal: impl Into<String>) -> CommandFormatPart {
    CommandFormatPart::Literal(literal.into())
}

/// Creates an `AnyText` part.
pub fn any_text_part() -> CommandFormatPart {
    CommandFormatPart::AnyText
}

/// Creates an `Entity` part.
pub fn entity_part(
    if_missing: impl Into<String>,
    validator: Option<EntityPartValidatorFn>,
) -> CommandFormatPart {
    CommandFormatPart::Entity {
        if_missing: if_missing.into(),
        validator,
    }
}

/// Creates a `Maybe` part.
pub fn maybe_part(part: CommandFormatPart) -> CommandFormatPart {
    CommandFormatPart::Maybe(Box::new(part))
}

/// Creates a `OneOf` part.
pub fn one_of_part(parts: NonEmpty<CommandFormatPart>) -> CommandFormatPart {
    CommandFormatPart::OneOf(Box::new(parts))
}

/// Marker trait for types that represent the type of a command part.
/// This exists so `CommandPartId`s can be associated with a certain type of part, so the correct thing can be returned when the parsed values are retrieved.
pub trait CommandPartType {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnyTextPartType;
impl CommandPartType for AnyTextPartType {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityPartType;
impl CommandPartType for EntityPartType {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaybePartType;
impl CommandPartType for MaybePartType {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneOfPartType;
impl CommandPartType for OneOfPartType {}

/// An identifier for a part of a command to be used to retrieve the parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPartId<T: CommandPartType>(String, PhantomData<fn(T)>);

impl<T: CommandPartType> CommandPartId<T> {
    /// Creates a new part ID.
    pub fn new(value: impl Into<String>) -> CommandPartId<T> {
        CommandPartId(value.into(), PhantomData)
    }
}

impl CommandFormat {
    /// Creates a format starting with a `Literal` part.
    pub fn new_with_literal(literal: impl Into<String>) -> CommandFormat {
        CommandFormat(NonEmpty::new((None, literal_part(literal))))
    }

    /// Creates a format starting with an `AnyText` part.
    pub fn new_with_any_text(id: CommandPartId<AnyTextPartType>) -> CommandFormat {
        CommandFormat(NonEmpty::new((Some(id.into()), any_text_part())))
    }

    /// Creates a format starting with an `Entity` part.
    pub fn new_with_entity(
        id: CommandPartId<EntityPartType>,
        if_missing: impl Into<String>,
        validator: Option<EntityPartValidatorFn>,
    ) -> CommandFormat {
        CommandFormat(NonEmpty::new((
            Some(id.into()),
            entity_part(if_missing, validator),
        )))
    }

    /// Creates a format starting with a `Maybe` part.
    pub fn new_with_maybe(
        id: CommandPartId<MaybePartType>,
        part: CommandFormatPart,
    ) -> CommandFormat {
        CommandFormat(NonEmpty::new((Some(id.into()), maybe_part(part))))
    }

    /// Creates a format starting with a `OneOf` part.
    pub fn new_with_one_of(
        id: CommandPartId<OneOfPartType>,
        parts: NonEmpty<CommandFormatPart>,
    ) -> CommandFormat {
        CommandFormat(NonEmpty::new((Some(id.into()), one_of_part(parts))))
    }

    /// Adds a `Literal` part to the format.
    pub fn then_literal(mut self, literal: impl Into<String>) -> Self {
        self.add_part(None, literal_part(literal));
        self
    }

    /// Adds an `AnyText` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_any_text(mut self, id: CommandPartId<AnyTextPartType>) -> Self {
        self.add_part(Some(id.into()), any_text_part());
        self
    }

    /// Adds an `Entity` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_entity(
        mut self,
        id: CommandPartId<EntityPartType>,
        if_missing: impl Into<String>,
        validator: Option<EntityPartValidatorFn>,
    ) -> Self {
        self.add_part(Some(id.into()), entity_part(if_missing, validator));
        self
    }

    /// Adds a `Maybe` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_maybe(mut self, id: CommandPartId<MaybePartType>, part: CommandFormatPart) -> Self {
        self.add_part(Some(id.into()), maybe_part(part));
        self
    }

    /// Adds a `OneOf` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_one_of(
        mut self,
        id: CommandPartId<OneOfPartType>,
        parts: NonEmpty<CommandFormatPart>,
    ) -> Self {
        self.add_part(Some(id.into()), one_of_part(parts));
        self
    }

    /// Adds a part to the format.
    /// Panics if `id` is `Some` and there is already a part with the same ID.
    fn add_part(&mut self, id: Option<TypedCommandPartId>, part: CommandFormatPart) {
        if let Some(id) = &id {
            if self
                .0
                .iter()
                .any(|(existing_id, _)| existing_id.as_ref() == Some(id))
            {
                panic!("Duplicate command part ID: {id:?}")
            }
        }

        self.0.push((id, part));
    }
}

/// An error encountered while parsing input into a command.
/// TODO rename to `CommandParseError`
pub enum CommandFormatParseError {}

pub struct ParsedCommand {
    //TODO
}

impl CommandFormat {
    /// Attempts to parse the provided input with this format.
    pub fn parse(&self, input: &str) -> Result<ParsedCommand, CommandFormatParseError> {
        todo!() //TODO
    }
}

pub struct CommandPartParseContext<T> {
    //TODO make this a reference?
    parsed_parts: Vec<ParsedCommandPart>,
    //TODO make this a reference?
    current_part: CommandFormatPart,
    target: T,
}

struct ParsedCommandPart {
    //TODO
}

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
                Some(TypedCommandPartId::Maybe(CommandPartId::new(
                    "optionalPartId"
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
