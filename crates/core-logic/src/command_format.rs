use bevy_ecs::prelude::*;

use nonempty::{nonempty, NonEmpty};

/// The format of a command a player can enter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFormat(NonEmpty<(Option<CommandPartId>, CommandFormatPart)>);

pub type EntityPartValidatorFn = fn(CommandPartParseContext<Entity>, &World) -> bool;

/// A piece of a command format.
/// TODO add variant for any text (like for `say`)
/// TODO allow parts to be optional (like the closing quote in `say`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandFormatPart {
    /// A literal value.
    Literal(String),
    /// A string that identifies an entity.
    Entity {
        /// The string to include in the error message if this part is missing (e.g. "what", "who", etc.)
        if_missing: String,
        /// The function to use to check whether the chosen Entity is valid.
        /// If this returns `false`, parsing will fail.
        validator: Option<EntityPartValidatorFn>,
    },
    /// One of the provided part types.
    /// Matching will be attempted in order.
    /// TODO allow some way to tell which one was matched after parsing
    OneOf(Box<NonEmpty<CommandFormatPart>>),
}

/// Creates a `Literal` part.
pub fn literal_part(literal: impl Into<String>) -> CommandFormatPart {
    CommandFormatPart::Literal(literal.into())
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

/// Creates a `OneOf` part.
pub fn one_of_part(parts: NonEmpty<CommandFormatPart>) -> CommandFormatPart {
    CommandFormatPart::OneOf(Box::new(parts))
}

/// An identifier for a part of a command to be used to retrieve the parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPartId(String);

impl CommandPartId {
    /// Creates a new ID.
    pub fn new(value: impl Into<String>) -> CommandPartId {
        CommandPartId(value.into())
    }
}

impl CommandFormat {
    /// Creates a format starting with a `Literal` part.
    pub fn new_with_literal(literal: impl Into<String>) -> CommandFormat {
        CommandFormat(NonEmpty::new((None, literal_part(literal))))
    }

    /// Creates a format starting with an `Entity` part.
    pub fn new_with_entity(
        id: CommandPartId,
        if_missing: impl Into<String>,
        validator: Option<EntityPartValidatorFn>,
    ) -> CommandFormat {
        CommandFormat(NonEmpty::new((
            Some(id),
            entity_part(if_missing, validator),
        )))
    }

    /// Creates a format starting with a `OneOf` part.
    pub fn new_with_one_of(id: CommandPartId, parts: NonEmpty<CommandFormatPart>) -> CommandFormat {
        CommandFormat(NonEmpty::new((Some(id), one_of_part(parts))))
    }

    /// Adds a `Literal` part to the format.
    pub fn then_literal(mut self, literal: impl Into<String>) -> Self {
        self.add_part(None, literal_part(literal));
        self
    }

    /// Adds an `Entity` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_entity(
        mut self,
        id: CommandPartId,
        if_missing: impl Into<String>,
        validator: Option<EntityPartValidatorFn>,
    ) -> Self {
        self.add_part(Some(id), entity_part(if_missing, validator));
        self
    }

    /// Adds a `OneOf` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_one_of(mut self, id: CommandPartId, parts: NonEmpty<CommandFormatPart>) -> Self {
        self.add_part(Some(id), one_of_part(parts));
        self
    }

    /// Adds a part to the format.
    /// Panics if `id` is `Some` and there is already a part with the same ID.
    fn add_part(&mut self, id: Option<CommandPartId>, part: CommandFormatPart) {
        if let Some(id) = &id {
            if self
                .0
                .iter()
                .any(|(existing_id, _)| existing_id.as_ref() == Some(id))
            {
                panic!("Duplicate command part ID: {}", id.0)
            }
        }

        self.0.push((id, part));
    }
}

pub struct CommandPartParseContext<T> {
    //TODO make this a reference?
    parsed_parts: ParsedParts,
    //TODO make this a reference?
    current_part: CommandFormatPart,
    target: T,
}

struct ParsedParts {
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
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![literal_part("option 1"), literal_part("option 2")],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(CommandPartId::new("entityPartId")),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: None,
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(CommandPartId::new("oneOfPartId")),
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
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![literal_part("option 1"), literal_part("option 2")],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(CommandPartId::new("entityPartId")),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: Some(entity_validator_fn),
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(CommandPartId::new("oneOfPartId")),
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
                Some(CommandPartId::new("entityPartId")),
                CommandFormatPart::Entity {
                    if_missing: "what".to_string(),
                    validator: None,
                }
            ),
            (None, CommandFormatPart::Literal("third part".to_string())),
            (
                Some(CommandPartId::new("oneOfPartId")),
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
