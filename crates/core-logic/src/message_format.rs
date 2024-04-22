use std::{collections::HashMap, marker::PhantomData};

use bevy_ecs::prelude::*;

pub struct MessageFormat<T: MessageTokens>(String, PhantomData<fn(T)>);

pub trait MessageTokens {
    fn get_tokens(&self) -> HashMap<String, Entity>;
}

impl<T: MessageTokens> MessageFormat<T> {
    /// Creates a `MessageFormat` with the provided format string.
    ///
    /// TODO explain format syntax
    pub fn new(format_string: String) -> MessageFormat<T> {
        MessageFormat(format_string, PhantomData)
    }

    /// Produces an interpolated string using the provided tokens.
    pub fn interpolate(&self, tokens: T, world: &World) -> String {
        todo!() //TODO
    }
}
