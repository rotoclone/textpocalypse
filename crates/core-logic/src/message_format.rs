use std::{collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till1, take_until, take_until1},
    character::complete::alphanumeric1,
    combinator::{eof, rest},
    multi::many0,
    sequence::{delimited, separated_pair},
    IResult,
};

use crate::{Description, Pronouns};

/// A message with places for interpolated values, such as entity names.
pub struct MessageFormat<T: MessageTokens>(Vec<MessageFormatChunk>, PhantomData<fn(T)>);

/// Trait for providing tokens for message interpolation.
pub trait MessageTokens {
    /// Returns a map of token names to the entities to use to fill in the interpolated values.
    fn get_token_map(&self) -> HashMap<String, Entity>;
}

/// An error during message interpolation.
#[derive(Debug)]
pub enum InterpolationError {
    /// A token in the format string has no matching entity provided.
    MissingToken(String),
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
    ///
    /// Token names must be alphanumeric.
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

enum MessageFormatChunk {
    String(String),
    Token { name: String, token_type: TokenType },
}

#[derive(Debug, PartialEq, Eq)]
enum TokenType {
    Name,
    PersonalSubjectPronoun,
    PersonalObjectPronoun,
    PossessivePronoun,
    PossessiveAdjectivePronoun,
    ReflexivePronoun,
    PluralSingular { plural: String, singular: String },
}

#[derive(Debug)]
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
    alt((parse_token_chunk, parse_non_token_chunk))(input)
}

fn parse_non_token_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    let (remaining, matched) = alt((take_until1(TOKEN_START), rest))(input)?;

    Ok((remaining, MessageFormatChunk::String(matched.to_string())))
}

fn parse_token_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    let (remaining, (token_name, token_type)) = parse_token(input)?;

    Ok((
        remaining,
        MessageFormatChunk::Token {
            name: token_name.to_string(),
            token_type,
        },
    ))
}

fn parse_token(input: &str) -> IResult<&str, (&str, TokenType)> {
    delimited(
        tag(TOKEN_START),
        separated_pair(alphanumeric1, tag(TOKEN_TYPE_SEPARATOR), parse_token_type),
        tag(TOKEN_END),
    )(input)
}

fn parse_token_type(input: &str) -> IResult<&str, TokenType> {
    let (remaining, token_type_string) = take_until(TOKEN_END)(input)?;
    let token_type = match token_type_string {
        "name" => TokenType::Name,
        "they" => TokenType::PersonalSubjectPronoun,
        "them" => TokenType::PersonalObjectPronoun,
        "theirs" => TokenType::PossessivePronoun,
        "their" => TokenType::PossessiveAdjectivePronoun,
        "themself" => TokenType::ReflexivePronoun,
        x => return parse_plural_singular_token_type(x),
    };

    Ok((remaining, token_type))
}

fn parse_plural_singular_token_type(input: &str) -> IResult<&str, TokenType> {
    let (remaining, (plural, singular)) = separated_pair(
        take_until(PLURAL_SINGULAR_SEPARATOR),
        tag(PLURAL_SINGULAR_SEPARATOR),
        rest,
    )(input)?;

    Ok((
        remaining,
        TokenType::PluralSingular {
            plural: plural.to_string(),
            singular: singular.to_string(),
        },
    ))
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
            MessageFormatChunk::Token { name, token_type } => {
                if let Some(entity) = tokens.get_token_map().get(name) {
                    Ok(token_type.interpolate(*entity, pov_entity, world))
                } else {
                    Err(InterpolationError::MissingToken(name.to_string()))
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
            TokenType::PersonalSubjectPronoun => Pronouns::get_personal_subject(entity, world),
            TokenType::PersonalObjectPronoun => Pronouns::get_personal_object(entity, world),
            TokenType::PossessivePronoun => Pronouns::get_possessive(entity, world),
            TokenType::PossessiveAdjectivePronoun => {
                Pronouns::get_possessive_adjective(entity, world)
            }
            TokenType::ReflexivePronoun => Pronouns::get_reflexive(entity, world),
            TokenType::PluralSingular { plural, singular } => {
                if Pronouns::is_plural(entity, world) {
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

    struct TestTokens(HashMap<String, Entity>);

    impl MessageTokens for TestTokens {
        fn get_token_map(&self) -> HashMap<String, Entity> {
            self.0.clone()
        }
    }

    #[test]
    fn parse_token_valid() {
        let input = "${entity1.name}";

        assert_eq!(
            ("", ("entity1", TokenType::Name)),
            parse_token(input).unwrap()
        );
    }

    #[test]
    fn parse_token_valid_plural_forms() {
        let input = "${entity1.eat/eats}";

        assert_eq!(
            (
                "",
                (
                    "entity1",
                    TokenType::PluralSingular {
                        plural: "eat".to_string(),
                        singular: "eats".to_string()
                    }
                )
            ),
            parse_token(input).unwrap()
        );
    }

    #[test]
    fn interpolate_empty() {
        let format = MessageFormat::new("").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = TestTokens(HashMap::new());

        assert_eq!("", format.interpolate(pov_entity, &tokens, &world).unwrap());
    }

    #[test]
    fn interpolate_no_tokens() {
        let format = MessageFormat::new("oh hello there").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let tokens = TestTokens(HashMap::new());

        assert_eq!(
            "oh hello there",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_just_token() {
        let format = MessageFormat::new("${entity1.name}").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                name: "some entity".to_string(),
                room_name: "some entity room name".to_string(),
                plural_name: "some entities".to_string(),
                article: Some("a".to_string()),
                pronouns: Pronouns::it(),
                aliases: vec![],
                description: "it's an entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = TestTokens([("entity1".to_string(), entity_1)].into());

        assert_eq!(
            "the some entity",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }

    #[test]
    fn interpolate_token_at_beginning() {
        let format = MessageFormat::new("${entity1.name} and stuff").unwrap();

        let mut world = World::new();
        let pov_entity = world.spawn_empty().id();
        let entity_1 = world
            .spawn(Description {
                name: "some entity".to_string(),
                room_name: "some entity room name".to_string(),
                plural_name: "some entities".to_string(),
                article: Some("a".to_string()),
                pronouns: Pronouns::it(),
                aliases: vec![],
                description: "it's an entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = TestTokens([("entity1".to_string(), entity_1)].into());

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
        let entity_1 = world
            .spawn(Description {
                name: "some entity".to_string(),
                room_name: "some entity room name".to_string(),
                plural_name: "some entities".to_string(),
                article: Some("a".to_string()),
                pronouns: Pronouns::it(),
                aliases: vec![],
                description: "it's an entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = TestTokens([("entity1".to_string(), entity_1)].into());

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
        let entity_1 = world
            .spawn(Description {
                name: "some entity".to_string(),
                room_name: "some entity room name".to_string(),
                plural_name: "some entities".to_string(),
                article: Some("a".to_string()),
                pronouns: Pronouns::it(),
                aliases: vec![],
                description: "it's an entity wow".to_string(),
                attribute_describers: vec![],
            })
            .id();
        let tokens = TestTokens([("entity1".to_string(), entity_1)].into());

        assert_eq!(
            "stuff and the some entity wow",
            format.interpolate(pov_entity, &tokens, &world).unwrap()
        );
    }
}
