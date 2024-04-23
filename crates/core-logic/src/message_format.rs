use std::{collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;

pub struct MessageFormat<T: MessageTokens>(String, PhantomData<fn(T)>);

pub trait MessageTokens {
    fn get_token_map(&self) -> HashMap<String, Entity>;
}

impl<T: MessageTokens> MessageFormat<T> {
    /// Creates a `MessageFormat` with the provided format string.
    ///
    /// Places for tokens in the format string are denoted by `%`.
    /// Tokens can be in the following formats:
    /// * `%name.type%`, where `name` is the name of the token, and `type` is one of the following types:
    ///   * `name`: the entity's name
    ///   * `they`: the entity's personal subject pronoun
    ///   * `them`: the entity's personal object pronoun
    ///   * `theirs`: the entity's possessive pronoun
    ///   * `their`: the entity's possessive adjective pronoun
    ///   * `themself`: the entity's reflexive pronoun
    /// * `%name:a/b%`, where `name` is the name of the token, `a` is the text to use if the entity's pronouns are plural, and `b` is the text to use if the entity's pronouns are singular
    ///
    /// Token names must be alphanumeric.
    ///
    /// An example format string: `%attacker.name% throws %object.name%, but %target.name% moves out of the way just before %object.they% %object:hit/hits% %target.them%.`
    /// This format string might produce the following result from `interpolate`: "Bob throws the rock, but Fred moves out of the way just before it hits him."
    pub fn new(format_string: String) -> MessageFormat<T> {
        MessageFormat(format_string, PhantomData)
    }

    /// Produces an interpolated string using the provided tokens.
    pub fn interpolate(&self, tokens: T, world: &World) -> String {
        todo!() //TODO
    }
}
