use nonempty::{nonempty, NonEmpty};

/// The format of a command a player can enter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFormat(NonEmpty<(Option<CommandPartId>, CommandFormatPart)>);

/// A piece of a command format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandFormatPart {
    /// A literal value.
    Literal(String),
    /// A string that identifies an entity.
    //TODO allow specifying limitations for what this entity can be? maybe that should be done in a separate validation step though
    Entity,
    /// One of the provided part types.
    /// Matching will be attempted in order.
    OneOf(Box<NonEmpty<CommandFormatPart>>),
}

/// An identifier for a part of a command to be used to retrieve the parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPartId(String);

impl CommandPartId {
    /// Creates a new ID.
    pub fn new<T: Into<String>>(value: T) -> CommandPartId {
        CommandPartId(value.into())
    }
}

impl CommandFormat {
    /// Creates a format starting with a `Literal` part.
    pub fn new_with_literal<T: Into<String>>(literal: T) -> CommandFormat {
        CommandFormat(NonEmpty::new((
            None,
            CommandFormatPart::Literal(literal.into()),
        )))
    }

    /// Creates a format starting with an `Entity` part.
    pub fn new_with_entity(id: CommandPartId) -> CommandFormat {
        CommandFormat(NonEmpty::new((Some(id), CommandFormatPart::Entity)))
    }

    /// Creates a format starting with a `OneOf` part.
    pub fn new_with_one_of(id: CommandPartId, parts: NonEmpty<CommandFormatPart>) -> CommandFormat {
        CommandFormat(NonEmpty::new((
            Some(id),
            CommandFormatPart::OneOf(Box::new(parts)),
        )))
    }

    /// Adds a `Literal` part to the format.
    pub fn then_literal<T: Into<String>>(mut self, literal: T) -> Self {
        self.add_part(None, CommandFormatPart::Literal(literal.into()));
        self
    }

    /// Adds an `Entity` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_entity(mut self, id: CommandPartId) -> Self {
        self.add_part(Some(id), CommandFormatPart::Entity);
        self
    }

    /// Adds a `OneOf` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_one_of(mut self, id: CommandPartId, parts: NonEmpty<CommandFormatPart>) -> Self {
        self.add_part(Some(id), CommandFormatPart::OneOf(Box::new(parts)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        let format = CommandFormat::new_with_literal("first part")
            .then_entity(CommandPartId::new("entityPartId"))
            .then_literal("third part")
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::Literal("option 2".to_string())
                ],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(CommandPartId::new("entityPartId")),
                CommandFormatPart::Entity
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
            .then_entity(CommandPartId::new("somePartId"))
            .then_literal("third part")
            .then_one_of(
                CommandPartId::new("somePartId"),
                nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::Literal("option 2".to_string())
                ],
            );
    }

    #[test]
    fn format_nested_one_of() {
        let format = CommandFormat::new_with_literal("first part")
            .then_entity(CommandPartId::new("entityPartId"))
            .then_literal("third part")
            .then_one_of(
                CommandPartId::new("oneOfPartId"),
                nonempty![
                    CommandFormatPart::Literal("option 1".to_string()),
                    CommandFormatPart::OneOf(Box::new(nonempty![
                        CommandFormatPart::Literal("option 2.1".to_string()),
                        CommandFormatPart::Literal("option 2.2".to_string())
                    ])),
                ],
            );

        let expected = CommandFormat(nonempty![
            (None, CommandFormatPart::Literal("first part".to_string())),
            (
                Some(CommandPartId::new("entityPartId")),
                CommandFormatPart::Entity
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
