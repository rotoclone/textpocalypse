use std::collections::HashMap;

use bevy_ecs::prelude::*;

pub struct MessageFormat(String);

impl MessageFormat {
    /// Creates a `MessageFormat` with the provided format string.
    ///
    /// TODO explain format syntax
    pub fn new(format_string: String) -> MessageFormat {
        MessageFormat(format_string)
    }

    /// Produces an interpolated string using the provided tokens.
    pub fn interpolate(&self, tokens: HashMap<String, Entity>, world: &World) -> String {
        todo!() //TODO
    }
}
