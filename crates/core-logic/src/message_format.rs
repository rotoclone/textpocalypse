use std::{collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use nom::{
    bytes::complete::{is_not, tag},
    character::complete::alphanumeric1,
    sequence::{delimited, separated_pair},
    IResult,
};

use crate::{Description, Pronouns};

/// A message with places for interpolated values, such as entity names.
pub struct MessageFormat<T: MessageTokens>(String, PhantomData<fn(T)>);

/// Trait for providing tokens for message interpolation.
pub trait MessageTokens {
    /// Returns a map of token names to the entities to use to fill in the interpolated values.
    fn get_token_map(&self) -> HashMap<String, Entity>;
}

/// An error during message interpolation.
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
    pub fn new(format_string: String) -> MessageFormat<T> {
        MessageFormat(format_string, PhantomData)
    }

    /// Produces an interpolated string to display to `pov_entity` using the provided tokens, or `Err` if the interpolation failed.
    pub fn interpolate(
        &self,
        pov_entity: Entity,
        tokens: &T,
        world: &World,
    ) -> Result<String, InterpolationError> {
        let parsed_format = ParsedMessageFormat::from(&self.0);
        let mut interpolated_chunks = Vec::new();
        for chunk in parsed_format.0 {
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

enum TokenType {
    Name,
    PersonalSubjectPronoun,
    PersonalObjectPronoun,
    PossessivePronoun,
    PossessiveAdjectivePronoun,
    ReflexivePronoun,
    PluralSingular { plural: String, singular: String },
}

impl ParsedMessageFormat {
    fn from(format_string: &str) -> ParsedMessageFormat {
        todo!() //TODO
    }
}

const TOKEN_START: &str = "${";
const TOKEN_END: &str = "}";
const PLURAL_SINGULAR_SEPARATOR: &str = "/";
const TOKEN_TYPE_SEPARATOR: &str = ".";

fn parse_plural_singular_token_type(input: &str) -> IResult<&str, TokenType> {
    let (remaining, (plural, singular)) = separated_pair(
        is_not(PLURAL_SINGULAR_SEPARATOR),
        tag(PLURAL_SINGULAR_SEPARATOR),
        is_not(""),
    )(input)?;

    Ok((
        remaining,
        TokenType::PluralSingular {
            plural: plural.to_string(),
            singular: singular.to_string(),
        },
    ))
}

fn parse_token_type(input: &str) -> IResult<&str, TokenType> {
    let token_type = match input {
        "name" => TokenType::Name,
        "they" => TokenType::PersonalSubjectPronoun,
        "them" => TokenType::PersonalObjectPronoun,
        "theirs" => TokenType::PossessivePronoun,
        "their" => TokenType::PossessiveAdjectivePronoun,
        "themself" => TokenType::ReflexivePronoun,
        x => return parse_plural_singular_token_type(x),
    };

    Ok(("", token_type))
}

fn parse_token(input: &str) -> IResult<&str, (&str, TokenType)> {
    delimited(
        tag(TOKEN_START),
        separated_pair(alphanumeric1, tag(TOKEN_TYPE_SEPARATOR), parse_token_type),
        tag(TOKEN_END),
    )(input)
}

fn parse_chunk(input: &str) -> IResult<&str, MessageFormatChunk> {
    todo!() //TODO
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
