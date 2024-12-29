use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
};

use bevy_ecs::prelude::*;

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
struct UntypedCommandFormatPart {
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

trait ParsePart<T>: ParsePartUntyped + ParsePartClone<T> {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<T>;

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped>;
}

trait ParsePartUntyped: std::fmt::Debug + ParsePartUntypedClone {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>>;
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
trait ParsePartUntypedClone {
    fn clone_box(&self) -> Box<dyn ParsePartUntyped>;
}

impl<T: 'static + ParsePartUntyped + Clone> ParsePartUntypedClone for T {
    fn clone_box(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
trait ParsePartClone<T> {
    fn clone_box(&self) -> Box<dyn ParsePart<T>>;
}

impl<T: 'static + ParsePart<P> + Clone, P> ParsePartClone<P> for T {
    fn clone_box(&self) -> Box<dyn ParsePart<P>> {
        Box::new(self.clone())
    }
}

trait ValidateParsedValue<T>: ValidateParsedValueUntyped + ValidateParsedValueClone<T> {
    fn validate(
        &self,
        context: PartValidatorContext<T>,
        world: &World,
    ) -> CommandPartValidateResult;

    fn as_untyped(&self) -> Box<dyn ValidateParsedValueUntyped>;
}

trait ValidateParsedValueUntyped: std::fmt::Debug + ValidateParsedValueUntypedClone {
    fn validate(
        &self,
        context: PartValidatorContext<Box<dyn Any>>,
        world: &World,
    ) -> CommandPartValidateResult;
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
trait ValidateParsedValueUntypedClone {
    fn clone_box(&self) -> Box<dyn ValidateParsedValueUntyped>;
}

impl<T: 'static + ValidateParsedValueUntyped + Clone> ValidateParsedValueUntypedClone for T {
    fn clone_box(&self) -> Box<dyn ValidateParsedValueUntyped> {
        Box::new(self.clone())
    }
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
trait ValidateParsedValueClone<T> {
    fn clone_box(&self) -> Box<dyn ValidateParsedValue<T>>;
}

impl<T: 'static + ValidateParsedValue<P> + Clone, P> ValidateParsedValueClone<P> for T {
    fn clone_box(&self) -> Box<dyn ValidateParsedValue<P>> {
        Box::new(self.clone())
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

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct CommandFormatPartOptions {
    /// The string to include in the error message if this part is missing (e.g. "what", "who", etc.)
    if_missing: Option<String>,
    /// The string to include in the format string for this part (e.g. "thing", "target", etc.)
    name_for_format_string: Option<String>,
}

pub struct PartParserContext {
    input: String,
    performing_entity: Entity,
}

pub enum CommandPartParseResult<T> {
    Success {
        parsed: T,
        remaining: String,
    },
    Failure {
        error: CommandPartParseError,
        remaining: String,
    },
}

impl<T: 'static> CommandPartParseResult<T> {
    /// Converts the generic type on this result to `Box<dyn Any>`, to make implementing `ParsePartUntyped` easier.
    pub fn into_generic(self) -> CommandPartParseResult<Box<dyn Any>> {
        match self {
            CommandPartParseResult::Success { parsed, remaining } => {
                CommandPartParseResult::Success {
                    parsed: Box::new(parsed),
                    remaining,
                }
            }
            CommandPartParseResult::Failure { error, remaining } => {
                CommandPartParseResult::Failure { error, remaining }
            }
        }
    }
}

pub enum CommandPartParseError {
    //TODO
}

pub struct PartValidatorContext<T> {
    parsed_value: T,
    performing_entity: Entity,
}

pub enum CommandPartValidateResult {
    Valid,
    Invalid(CommandPartValidateError),
}

pub enum CommandPartValidateError {
    //TODO
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

#[derive(Debug, Clone)]
struct LiteralParser(String);

impl ParsePart<String> for LiteralParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<String> {
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

impl ParsePartUntyped for LiteralParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
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

#[derive(Debug, Clone, Copy)]
struct AnyTextParser;

impl ParsePart<String> for AnyTextParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<String> {
        // TODO how is this supposed to know when to stop?
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(*self)
    }
}

impl ParsePartUntyped for AnyTextParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
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

#[derive(Debug, Clone, Copy)]
struct EntityParser;

impl ParsePart<Entity> for EntityParser {
    fn parse(&self, context: PartParserContext, world: &World) -> CommandPartParseResult<Entity> {
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(*self)
    }
}

impl ParsePartUntyped for EntityParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
    }
}

/// Creates a part to maybe consume something.
pub fn maybe_part<T: 'static + std::fmt::Debug + Clone>(
    id: CommandPartId<Option<T>>,
    part: CommandFormatPart<T>,
) -> CommandFormatPart<Option<T>> {
    CommandFormatPart {
        id: Some(id),
        options: CommandFormatPartOptions::default(),
        parser: Box::new(MaybeParser(part)),
        validator: None,
    }
}

#[derive(Debug, Clone)]
struct MaybeParser<T: Clone>(CommandFormatPart<T>);

impl<T: 'static + std::fmt::Debug + Clone> ParsePart<Option<T>> for MaybeParser<T> {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Option<T>> {
        match self.0.parser.parse(context, world) {
            CommandPartParseResult::Success { parsed, remaining } => {
                CommandPartParseResult::Success {
                    parsed: Some(parsed),
                    remaining,
                }
            }
            CommandPartParseResult::Failure { remaining, .. } => CommandPartParseResult::Success {
                parsed: None,
                remaining,
            },
        }
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(MaybeParser(self.0.clone()))
    }
}

impl<T: 'static + std::fmt::Debug + Clone> ParsePartUntyped for MaybeParser<T> {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
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

#[derive(Debug, Clone)]
struct OneOfParser(NonEmpty<UntypedCommandFormatPart>);

impl ParsePart<Box<dyn Any>> for OneOfParser {
    fn parse(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        todo!() //TODO
    }

    fn as_untyped(&self) -> Box<dyn ParsePartUntyped> {
        Box::new(self.clone())
    }
}

impl ParsePartUntyped for OneOfParser {
    fn parse_untyped(
        &self,
        context: PartParserContext,
        world: &World,
    ) -> CommandPartParseResult<Box<dyn Any>> {
        self.parse(context, world).into_generic()
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
    /// Creates a format starting with a `Literal` part.
    pub fn new_with_literal(literal: impl Into<String>) -> CommandFormat {
        CommandFormat(NonEmpty::new(literal_part(literal).into()))
    }

    /// Creates a format starting with an `AnyText` part.
    pub fn new_with_any_text(id: CommandPartId<String>) -> CommandFormat {
        CommandFormat(NonEmpty::new(any_text_part(id).into()))
    }

    /// Creates a format starting with an `Entity` part.
    pub fn new_with_entity(id: CommandPartId<Entity>) -> CommandFormat {
        CommandFormat(NonEmpty::new(entity_part(id).into()))
    }

    /// Creates a format starting with a `Maybe` part.
    pub fn new_with_maybe<T: 'static + std::fmt::Debug + Clone>(
        id: CommandPartId<Option<T>>,
        part: CommandFormatPart<T>,
    ) -> CommandFormat {
        CommandFormat(NonEmpty::new(maybe_part(id, part).into()))
    }

    /// Creates a format starting with a `OneOf` part.
    pub fn new_with_one_of(parts: NonEmpty<UntypedCommandFormatPart>) -> CommandFormat {
        CommandFormat(NonEmpty::new(one_of_part(parts).into()))
    }

    /// Adds a `Literal` part to the format.
    pub fn then_literal(mut self, literal: impl Into<String>) -> Self {
        self.add_part(literal_part(literal).into());
        self
    }

    /// Adds an `AnyText` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_any_text(mut self, id: CommandPartId<String>) -> Self {
        self.add_part(any_text_part(id).into());
        self
    }

    /// Adds an `Entity` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_entity(mut self, id: CommandPartId<Entity>) -> Self {
        self.add_part(entity_part(id).into());
        self
    }

    /// Adds a `Maybe` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_maybe<T: 'static + std::fmt::Debug + Clone>(
        mut self,
        id: CommandPartId<Option<T>>,
        part: CommandFormatPart<T>,
    ) -> Self {
        self.add_part(maybe_part(id, part).into());
        self
    }

    /// Adds a `OneOf` part to the format.
    /// Panics if there is already a part with the provided ID.
    pub fn then_one_of(mut self, parts: NonEmpty<UntypedCommandFormatPart>) -> Self {
        self.add_part(one_of_part(parts).into());
        self
    }

    /// Adds a part to the format.
    /// Panics if `id` is `Some` and there is already a part with the same ID.
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
pub enum CommandFormatParseError {}

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

struct ParsedCommandPart {
    //TODO
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
