use bevy_ecs::prelude::*;

use std::{any::Any, borrow::Borrow, ops::Deref};

use crate::component::Description;

use super::PartParserContext;

/// A value parsed from a command.
#[derive(Debug, Clone)]
pub enum ParsedValue {
    String(String),
    Entity(Entity),
    //TODO is this necessary?
    Option(Option<Box<ParsedValue>>),
}

impl ParsedValue {
    /// Builds a string representing this value to use in a parsing error message.
    pub fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        match self {
            ParsedValue::String(s) => s.clone(),
            ParsedValue::Entity(e) => {
                Description::get_reference_name(*e, Some(context.entering_entity), world)
            }
            ParsedValue::Option(o) => o
                .as_ref()
                .map(|v| v.to_string_for_parse_error(context, world))
                .unwrap_or_default(),
        }
    }
}

pub trait TryIntoRef<T> {
    // TODO doc
    fn try_into_ref(&self) -> Option<&T>;
}

impl From<String> for ParsedValue {
    fn from(value: String) -> Self {
        ParsedValue::String(value)
    }
}

impl TryIntoRef<String> for ParsedValue {
    fn try_into_ref(&self) -> Option<&String> {
        if let ParsedValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

impl From<Entity> for ParsedValue {
    fn from(value: Entity) -> Self {
        ParsedValue::Entity(value)
    }
}

impl TryIntoRef<Entity> for ParsedValue {
    fn try_into_ref(&self) -> Option<&Entity> {
        if let ParsedValue::Entity(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

/* TODO remove?
impl From<Option<Box<ParsedValue>>> for ParsedValue {
    fn from(value: Option<Box<ParsedValue>>) -> Self {
        ParsedValue::Option(value)
    }
}
    */

impl<T: Into<ParsedValue>> From<Option<T>> for ParsedValue {
    fn from(value: Option<T>) -> Self {
        ParsedValue::Option(value.map(|v| Box::new(v.into())))
    }
}

impl TryIntoRef<Option<Box<ParsedValue>>> for ParsedValue {
    fn try_into_ref(&self) -> Option<&Option<Box<ParsedValue>>> {
        if let ParsedValue::Option(o) = self {
            Some(o)
        } else {
            None
        }
    }
}

impl<T> TryIntoRef<Option<T>> for ParsedValue
where
    ParsedValue: TryIntoRef<T>,
{
    fn try_into_ref(&self) -> Option<&Option<T>> {
        if let ParsedValue::Option(o) = self {
            o.map(|p| p.try_into_ref())
        } else {
            None
        }
    }
}

/* TODO remove

//TODO come up with a better name for this
#[derive(Debug)]
pub struct ParsedValueContainer {
    pub value: Box<dyn ParsedValue>,
    pub value_as_any: Box<dyn Any>,
}

pub trait ParsedValue: Any + std::fmt::Debug {
    /// Builds a string representing this value to use in a parsing error message.
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String;

    /// Converts to `Any` for downcasting
    /// TODO remove if `ParsedValueContainer` works
    fn as_any(&self) -> &dyn Any;
}

impl ParsedValue for String {
    fn to_string_for_parse_error(&self, _: PartParserContext, _: &World) -> String {
        self.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ParsedValue for Entity {
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        Description::get_reference_name(*self, Some(context.entering_entity), world)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: ParsedValue> ParsedValue for Option<T> {
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        self.as_ref()
            .map(|v| v.to_string_for_parse_error(context, world))
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: ?Sized + ParsedValue> ParsedValue for Box<T> {
    fn to_string_for_parse_error(&self, context: PartParserContext, world: &World) -> String {
        self.deref().to_string_for_parse_error(context, world)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
    */
