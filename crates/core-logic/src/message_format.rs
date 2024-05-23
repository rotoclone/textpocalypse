use std::{collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    multi::many0,
    sequence::{delimited, separated_pair},
    IResult,
};
use voca_rs::Voca;

use crate::{Description, Pronouns};

/// A message with places for interpolated values, such as entity names.
#[derive(Clone)]
pub struct MessageFormat<T: MessageTokens>(Vec<MessageFormatChunk>, PhantomData<fn(T)>);

/// The name of a token
#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct TokenName(String);

impl From<&str> for TokenName {
    fn from(value: &str) -> Self {
        TokenName(value.to_string())
    }
}

impl From<String> for TokenName {
    fn from(value: String) -> Self {
        TokenName(value)
    }
}

/// The value to use when interpolating a token
#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum TokenValue {
    String(String),
    Entity(Entity),
}

/// Trait for providing tokens for message interpolation.
pub trait MessageTokens {
    /// Returns a map of token names to what to use to fill in the interpolated values.
    fn get_token_map(&self) -> HashMap<TokenName, TokenValue>;
}

/// A generic set of tokens, for use with one-time message formats that don't need fancy token type safety.
pub struct BasicTokens(HashMap<TokenName, TokenValue>);

impl MessageTokens for BasicTokens {
    fn get_token_map(&self) -> HashMap<TokenName, TokenValue> {
        self.0.clone()
    }
}

impl BasicTokens {
    /// Creates an empty set of tokens.
    pub fn new() -> BasicTokens {
        BasicTokens(HashMap::new())
    }

    /// Adds a string token.
    pub fn with_string(self, name: TokenName, value: String) -> BasicTokens {
        self.with_token(name, TokenValue::String(value))
    }

    /// Adds an entity token.
    pub fn with_entity(self, name: TokenName, value: Entity) -> BasicTokens {
        self.with_token(name, TokenValue::Entity(value))
    }

    /// Adds a token, replacing any previous value with the same name.
    fn with_token(mut self, name: TokenName, value: TokenValue) -> BasicTokens {
        self.0.insert(name, value);
        self
    }
}

/// An error during message interpolation.
#[derive(Debug, PartialEq, Eq)]
pub enum InterpolationError {
    /// A token in the format string has no matching value provided.
    MissingToken(TokenName),
    /// A token was paired with the wrong type of value
    InvalidTokenValue(TokenName, TokenValue),
}

impl<T: MessageTokens> MessageFormat<T> {
    /// Creates a `MessageFormat` with the provided format string.
    ///
    /// Places for tokens in the format string are enclosed in `${}`.
    /// Tokens can be in the following formats:
    /// * `${name.type}`, where `name` is the name of the token, and `type` is one of the following types:
    ///   * `name`: the entity's name
    ///   * `they`: the entity's personal subject pronoun
    ///   * `them`: the entity's personal object pronoun
    ///   * `theirs`: the entity's possessive pronoun
    ///   * `their`: the entity's possessive adjective pronoun
    ///   * `themself`: the entity's reflexive pronoun
    /// * `${name.a/b}`, where `name` is the name of the token, `a` is the text to use if the entity's pronouns are plural, and `b` is the text to use if the entity's pronouns are singular
    /// * `${name}`, where `name` is the name of the token. This token will be simply the string associated with the token, not a value derived from an entity.
    ///
    /// For typed tokens, if the first character of the type of the token is capitalized (for example, `${someToken.They}`), then the interpolated value will have its first character capitalized.
    ///
    /// For plain tokens, if the first character of the name of the token is capitalized (for example, `${SomeToken}`), then the first letter of the token's value will have its first character capitalized.
    ///
    /// Token must be alphanumeric, but can contain underscores.
    ///
    /// An example format string: `${attacker.name} throws ${object.name}, but ${target.name} moves out of the way just before ${object.they} ${object.hit/hits} ${target.them}.`
    /// This format string might produce the following result from `interpolate`: "Bob throws the rock, but Fred moves out of the way just before it hits him."
    pub fn new(format_string: &str) -> Result<MessageFormat<T>, ParseError> {
        Ok(MessageFormat(
            MessageFormatChunk::parse(format_string)?,
            PhantomData,
        ))
    }

    /// Produces an interpolated string to display to `pov_entity` using the provided tokens, or `Err` if the interpolation failed.
    pub fn interpolate(
        &self,
        pov_entity: Entity,
        tokens: &T,
        world: &World,
    ) -> Result<String, InterpolationError> {
        let mut interpolated_chunks = Vec::new();
        for chunk in &self.0 {
            interpolated_chunks.push(chunk.interpolate(pov_entity, tokens, world)?);
        }
        Ok(interpolated_chunks.join(""))
    }
}

struct ParsedMessageFormat(Vec<MessageFormatChunk>);

#[derive(Debug, PartialEq, Eq, Clone)]
enum MessageFormatChunk {
    String(String),
    PlainToken {
        name: TokenName,
        capitalize: bool,
    },
    Token {
        name: TokenName,
        token_type: TokenType,
        capitalize: bool,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TokenType {
    Name,
    PersonalSubjectPronoun,
    PersonalObjectPronoun,
    PossessivePronoun,
    PossessiveAdjectivePronoun,
    ReflexivePronoun,
    PluralSingular { plural: String, singular: String },
}

#[derive(Debug, PartialEq)]
pub enum ParseError<'i> {
    /// Some of the input remained unparsed for some reason.
    /// The unparsed part is included.
    UnparsedInput(&'i str),
    /// Some other kind of thing went wrong with the parsing
    InternalParserError(nom::Err<nom::error::Error<&'i str>>),
}

impl MessageFormatChunk {
    /// Parses the provided format string into chunks.
    fn parse(format_string: &str) -> Result<Vec<MessageFormatChunk>, ParseError> {
        match many0(parse_chunk)(format_string) {
            Ok((remaining, chunks)) => {
                if !remaining.is_empty() {
                    return Err(ParseError::UnparsedInput(remaining));
                }
                Ok(chunks)
            }
            Err(e) => Err(ParseError::InternalParserError(e)),
        }
    }
}

const TOKEN_START: &str = "${";
const TOKEN_END: &str = "}";
const PLURAL_SINGULAR_SEPARATOR: &str = "/";
const TOKEN_TYPE_SEPARATOR: &str = ".";

fn parse_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    alt((
        parse_token_chunk,
        parse_plain_token_chunk,
        parse_string_chunk,
    ))(input)
}

fn parse_token_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    let (remaining, (token_name, (token_type, capitalize))) = parse_token(input)?;

    Ok((
        remaining,
        MessageFormatChunk::Token {
            name: TokenName(token_name.to_string()),
            token_type,
            capitalize,
        },
    ))
}

fn parse_token(input: &str) -> IResult<&str, (&str, (TokenType, bool))> {
    delimited(
        tag(TOKEN_START),
        separated_pair(
            take_while1(is_valid_token_name_char),
            tag(TOKEN_TYPE_SEPARATOR),
            parse_token_type,
        ),
        tag(TOKEN_END),
    )(input)
}

fn parse_token_type(input: &str) -> IResult<&str, (TokenType, bool)> {
    let (remaining, token_type_string) = take_until(TOKEN_END)(input)?;
    let token_type = match token_type_string {
        "name" | "Name" => TokenType::Name,
        "they" | "They" => TokenType::PersonalSubjectPronoun,
        "them" | "Them" => TokenType::PersonalObjectPronoun,
        "theirs" | "Theirs" => TokenType::PossessivePronoun,
        "their" | "Their" => TokenType::PossessiveAdjectivePronoun,
        "themself" | "Themself" => TokenType::ReflexivePronoun,
        _ => return parse_plural_singular_token_type(input),
    };

    let capitalize = token_type_string._is_upper_first();

    Ok((remaining, (token_type, capitalize)))
}

fn parse_plural_singular_token_type(input: &str) -> IResult<&str, (TokenType, bool)> {
    let (remaining, (plural, singular)) = separated_pair(
        take_until(PLURAL_SINGULAR_SEPARATOR),
        tag(PLURAL_SINGULAR_SEPARATOR),
        take_until(TOKEN_END),
    )(input)?;

    Ok((
        remaining,
        (
            TokenType::PluralSingular {
                plural: plural.to_string(),
                singular: singular.to_string(),
            },
            false,
        ),
    ))
}

fn parse_plain_token_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    let (remaining, token_name) = delimited(
        tag(TOKEN_START),
        take_while1(is_valid_token_name_char),
        tag(TOKEN_END),
    )(input)?;

    Ok((
        remaining,
        MessageFormatChunk::PlainToken {
            name: TokenName(token_name.to_string()),
            capitalize: token_name._is_upper_first(),
        },
    ))
}

fn parse_string_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    let (remaining, matched) = alt((take_until(TOKEN_START), take_while1(|_| true)))(input)?;

    Ok((remaining, MessageFormatChunk::String(matched.to_string())))
}

/// Checks whether a character is allowed to be part of a token name.
fn is_valid_token_name_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

impl MessageFormatChunk {
    /// Interpolates this chunk into a string to display to `pov_entity`.
    fn interpolate<T: MessageTokens>(
        &self,
        pov_entity: Entity,
        tokens: &T,
        world: &World,
    ) -> Result<String, InterpolationError> {
        match self {
            MessageFormatChunk::String(s) => Ok(s.to_string()),
            MessageFormatChunk::PlainToken { name, capitalize } => {
                //TODO handle plain tokens with uppercase first letters correctly
                if let Some(token_value) = tokens.get_token_map().get(name) {
                    if let TokenValue::String(s) = token_value {
                        if *capitalize {
                            Ok(s.clone()._upper_first())
                        } else {
                            Ok(s.clone())
                        }
                    } else {
                        Err(InterpolationError::InvalidTokenValue(
                            name.clone(),
                            token_value.clone(),
                        ))
                    }
                } else {
                    Err(InterpolationError::MissingToken(name.clone()))
                }
            }
            MessageFormatChunk::Token {
                name,
                token_type,
                capitalize,
            } => {
                if let Some(token_value) = tokens.get_token_map().get(name) {
                    if let TokenValue::Entity(e) = token_value {
                        //TODO capitalize if needed
                        Ok(token_type.interpolate(*e, pov_entity, world))
                    } else {
                        Err(InterpolationError::InvalidTokenValue(
                            name.clone(),
                            token_value.clone(),
                        ))
                    }
                } else {
                    Err(InterpolationError::MissingToken(name.clone()))
                }
            }
        }
    }
}

impl TokenType {
    /// Interpolates this token with values from `entity`, from the point of view of `pov_entity`.
    fn interpolate(&self, entity: Entity, pov_entity: Entity, world: &World) -> String {
        match self {
            TokenType::Name => Description::get_reference_name(entity, Some(pov_entity), world),
            TokenType::PersonalSubjectPronoun => {
                Pronouns::get_personal_subject(entity, Some(pov_entity), world)
            }
            TokenType::PersonalObjectPronoun => {
                Pronouns::get_personal_object(entity, Some(pov_entity), world)
            }
            TokenType::PossessivePronoun => {
                Pronouns::get_possessive(entity, Some(pov_entity), world)
            }
            TokenType::PossessiveAdjectivePronoun => {
                Pronouns::get_possessive_adjective(entity, Some(pov_entity), world)
            }
            TokenType::ReflexivePronoun => Pronouns::get_reflexive(entity, Some(pov_entity), world),
            TokenType::PluralSingular { plural, singular } => {
                // if this entity is the POV entity, it will be referred to elsewhere as "you", which is grammatically plural
                if entity == pov_entity || Pronouns::is_plural(entity, world) {
                    plural.to_string()
                } else {
                    singular.to_string()
                }
            }
        }
    }
}

mod tests {
    use super::*;

    #[allow(unused)]
    fn build_entity_1_description() -> Description {
        Description {
            name: "some entity".to_string(),
            room_name: "some entity room name".to_string(),
            plural_name: "some entities".to_string(),
            article: Some("a".to_string()),
            pronouns: Pronouns::it(),
            aliases: vec![],
            description: "it's an entity wow".to_string(),
            attribute_describers: vec![],
        }
    }

    #[test]
    fn interpolate_empty() {
        let format = MessageFormat::new("").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new();

        assert_eq!("", format.interpolate(pov_entity, &tokens, &world).unwrap());
    }

    #[test]
    fn interpolate_no_tokens() {
        let format = MessageFormat::new("oh hello there").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new();

        assert_eq!(
            "oh hello there",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_no_tokens_with_token_provided() {
        let format = MessageFormat::new("oh hello there").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new().with_string("hello".into(), "sup".to_string());

        assert_eq!(
            "oh hello there",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_special_characters() {
        let format = MessageFormat::new("oh hello there $a {{}{}{}}}}}}{{{{{ ./b./././").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new();

        assert_eq!(
            "oh hello there $a {{}{}{}}}}}}{{{{{ ./b./././",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_invalid_token_name() {
        assert!(matches!(
            MessageFormat::<BasicTokens>::new("${abc*}"),
            Err(ParseError::InternalParserError(_))
        ));
    }

    #[test]
    fn interpolate_invalid_token_type() {
        assert!(matches!(
            MessageFormat::<BasicTokens>::new("${entity1.florb}"),
            Err(ParseError::InternalParserError(_))
        ));
    }

    #[test]
    fn interpolate_empty_token() {
        assert!(matches!(
            MessageFormat::<BasicTokens>::new("${}"),
            Err(ParseError::InternalParserError(_))
        ));
    }

    #[test]
    fn interpolate_partial_token() {
        assert!(matches!(
            MessageFormat::<BasicTokens>::new("oh ${hello"),
            Err(ParseError::InternalParserError(_))
        ));
    }

    #[test]
    fn interpolate_plain_missing_token() {
        let format = MessageFormat::new("${somethin}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new();

        assert_eq!(
            Err(InterpolationError::MissingToken(TokenName(
                "somethin".to_string()
            ))),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_name_missing_token() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new();

        assert_eq!(
            Err(InterpolationError::MissingToken(TokenName(
                "entity1".to_string()
            ))),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_duplicate_token_names_different_value_types_string_provided() {
        let format = MessageFormat::new("${entity1.name} and ${entity1}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new().with_string("entity1".into(), "sup".to_string());

        assert_eq!(
            Err(InterpolationError::InvalidTokenValue(
                "entity1".into(),
                TokenValue::String("sup".to_string())
            )),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_duplicate_token_names_different_value_types_entity_provided() {
        let format = MessageFormat::new("${entity1.name} and ${entity1}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            Err(InterpolationError::InvalidTokenValue(
                "entity1".into(),
                TokenValue::Entity(entity_1)
            )),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_plain() {
        let format = MessageFormat::new("${somethin}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new().with_string("somethin".into(), "oh hello".to_string());

        assert_eq!(
            "oh hello",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_plain_wrong_token_value_type() {
        let format = MessageFormat::new("${somethin}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("somethin".into(), entity_1);

        assert_eq!(
            Err(InterpolationError::InvalidTokenValue(
                "somethin".into(),
                TokenValue::Entity(entity_1)
            )),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_name() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "the some entity",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_name_wrong_token_value_type() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = BasicTokens::new().with_string("entity1".into(), "oh hello".to_string());

        assert_eq!(
            Err(InterpolationError::InvalidTokenValue(
                "entity1".into(),
                TokenValue::String("oh hello".to_string())
            )),
            format.interpolate(pov_entity, &tokens, &world)
        );
    }

    #[test]
    fn interpolate_name_no_article() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                article: None,
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "some entity",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_name_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let entity_1 = world
            .spawn(Description {
                article: None,
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "you",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_personal_subject() {
        let format = MessageFormat::new("${entity1.they}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::new("p subj", "p obj", "poss", "poss adj", "refl", false),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "p subj",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_personal_object() {
        let format = MessageFormat::new("${entity1.them}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::new("p subj", "p obj", "poss", "poss adj", "refl", false),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "p obj",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive() {
        let format = MessageFormat::new("${entity1.theirs}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::new("p subj", "p obj", "poss", "poss adj", "refl", false),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "poss",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive_adjective() {
        let format = MessageFormat::new("${entity1.their}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::new("p subj", "p obj", "poss", "poss adj", "refl", false),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "poss adj",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive_reflexive() {
        let format = MessageFormat::new("${entity1.themself}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::new("p subj", "p obj", "poss", "poss adj", "refl", false),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "refl",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_personal_subject_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.they}").unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "you",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_personal_object_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.them}").unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "you",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.theirs}").unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "yours",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive_adjective_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.their}").unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "your",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_possessive_reflexive_same_as_pov_entity() {
        let format = MessageFormat::new("${entity1.themself}").unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "yourself",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_plural_forms_singular() {
        let format =
            MessageFormat::new("${entity1.this is the plural form/this is the singular form}")
                .unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "this is the singular form",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_plural_forms_plural() {
        let format =
            MessageFormat::new("${entity1.this is the plural form/this is the singular form}")
                .unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                pronouns: Pronouns::they(),
                ..build_entity_1_description()
            })
            .id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "this is the plural form",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_plural_forms_singular_same_as_pov_entity() {
        let format =
            MessageFormat::new("${entity1.this is the plural form/this is the singular form}")
                .unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "this is the plural form",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_token_at_beginning() {
        let format = MessageFormat::new("${entity1.name} and stuff").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "the some entity and stuff",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_token_at_end() {
        let format = MessageFormat::new("stuff and ${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "stuff and the some entity",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_token_in_middle() {
        let format = MessageFormat::new("stuff and ${entity1.name} wow").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let tokens = BasicTokens::new().with_entity("entity1".into(), entity_1);

        assert_eq!(
            "stuff and the some entity wow",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_multiple_tokens() {
        //TODO add some way to specify whether an entity's name is plural separate from its pronouns.
        // For example, if a person is named Bob and their pronouns are they/them, "<name> <are/is> here" should be "Bob is here", but "<personal subject> <are/is> here" should be "they are here".
        let format =
            MessageFormat::new("it's ${entity1.name} and ${entity1.they} ${entity1.are/is} ${a_string}. Oh hey and ${entity2.name} is here and ${entity2.they} ${entity2.are/is} cool too I guess.")
                .unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let entity_2 = world
            .spawn(Description {
                name: "some other entity".to_string(),
                room_name: "some other entity room name".to_string(),
                plural_name: "some other entities".to_string(),
                article: None,
                pronouns: Pronouns::they(),
                aliases: vec![],
                description: "it's a different entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = BasicTokens::new()
            .with_entity("entity1".into(), entity_1)
            .with_entity("entity2".into(), entity_2)
            .with_string("a_string".into(), "pretty_cool".to_string());

        assert_eq!(
            "it's the some entity and it is pretty cool. Oh hey and some other entity is here and they are cool too I guess.",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_multiple_tokens_same_as_pov_entity() {
        let format =
            MessageFormat::new("it's ${entity1.name} and ${entity1.they} ${entity1.are/is} ${a_string}. Oh hey and ${entity2.name} is here and ${entity2.they} ${entity2.are/is} cool too I guess.")
                .unwrap();

        let mut world = World::new();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let entity_2 = world
            .spawn(Description {
                name: "some other entity".to_string(),
                room_name: "some other entity room name".to_string(),
                plural_name: "some other entities".to_string(),
                article: None,
                pronouns: Pronouns::they(),
                aliases: vec![],
                description: "it's a different entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = BasicTokens::new()
            .with_entity("entity1".into(), entity_1)
            .with_entity("entity2".into(), entity_2)
            .with_string("a_string".into(), "pretty_cool".to_string());

        assert_eq!(
            "it's you and you are pretty cool. Oh hey and some other entity is here and they are cool too I guess.",
            format.interpolate(entity_1, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_multiple_tokens_no_non_token_parts() {
        let format =
            MessageFormat::new("${entity1.name}${entity1.they}${entity1.are/is}${a_string}")
                .unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world.spawn(build_entity_1_description()).id();
        let entity_2 = world
            .spawn(Description {
                name: "some other entity".to_string(),
                room_name: "some other entity room name".to_string(),
                plural_name: "some other entities".to_string(),
                article: None,
                pronouns: Pronouns::they(),
                aliases: vec![],
                description: "it's a different entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = BasicTokens::new()
            .with_entity("entity1".into(), entity_1)
            .with_entity("entity2".into(), entity_2)
            .with_string("a_string".into(), "pretty_cool".to_string());

        assert_eq!(
            "the some entityitispretty cool",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_multiple_tokens_with_invalid_token() {
        let format_string = "it's ${entity1.name} and ${entity1.they} ${entity1.are/is} ${a_string}. ${} Oh hey and ${entity2.name} is here and ${entity2.they} ${entity2.are/is} cool too I guess.";

        assert!(matches!(
            MessageFormat::<BasicTokens>::new(format_string),
            Err(ParseError::InternalParserError(_))
        ));
    }

    //TODO tests for capitalization
}
