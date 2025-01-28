use bevy_ecs::prelude::*;

use std::any::Any;

use super::PartParserContext;

pub trait ParseableValue: Any + std::fmt::Debug {
    /// Builds a string representing this value to use in a parsing error message.
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String;

    /// Converts to `Any` for downcasting
    fn as_any(&self) -> &dyn Any;
}

impl ParseableValue for String {
    fn to_string_for_parse_error(&self, _: PartParserContext, _: &World) -> String {
        self.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
