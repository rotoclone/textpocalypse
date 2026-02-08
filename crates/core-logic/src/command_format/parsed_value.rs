use bevy_ecs::prelude::*;

use crate::{command_format::CommandFormatPart, component::Description, Direction};

use super::PartParserContext;

/// A value parsed from a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedValue {
    String(String),
    Entity(Entity),
    Direction(Direction),
    Option(Option<Box<ParsedValue>>),
}

impl ParsedValue {
    /// Builds a string representing this value to use in a parsing error message.
    pub fn to_string_for_parse_error(
        &self,
        part: &CommandFormatPart,
        context: &PartParserContext,
        world: &World,
    ) -> String {
        match self {
            ParsedValue::String(s) => s.clone(),
            ParsedValue::Entity(e) => {
                Description::get_reference_name(*e, Some(context.entering_entity), world)
            }
            ParsedValue::Direction(d) => d.to_string(),
            ParsedValue::Option(o) => o
                .as_ref()
                .map(|v| v.to_string_for_parse_error(part, context, world))
                // an optional part that parsed to nothing is effectively unparsed, so use the `if_unparsed` value
                //TODO but this needs to respect the include in errors setting
                .or_else(|| part.options().if_unparsed.clone())
                .unwrap_or_default(),
        }
    }
}

impl From<String> for ParsedValue {
    fn from(value: String) -> Self {
        ParsedValue::String(value)
    }
}

impl TryFrom<ParsedValue> for String {
    type Error = ();

    fn try_from(value: ParsedValue) -> Result<Self, Self::Error> {
        if let ParsedValue::String(s) = value {
            Ok(s)
        } else {
            Err(())
        }
    }
}

impl From<Entity> for ParsedValue {
    fn from(value: Entity) -> Self {
        ParsedValue::Entity(value)
    }
}

impl TryFrom<ParsedValue> for Entity {
    type Error = ();

    fn try_from(value: ParsedValue) -> Result<Self, Self::Error> {
        if let ParsedValue::Entity(e) = value {
            Ok(e)
        } else {
            Err(())
        }
    }
}

impl From<Direction> for ParsedValue {
    fn from(value: Direction) -> Self {
        ParsedValue::Direction(value)
    }
}

impl TryFrom<ParsedValue> for Direction {
    type Error = ();

    fn try_from(value: ParsedValue) -> Result<Self, Self::Error> {
        if let ParsedValue::Direction(d) = value {
            Ok(d)
        } else {
            Err(())
        }
    }
}

impl<T> From<Option<T>> for ParsedValue
where
    T: Into<ParsedValue>,
{
    fn from(value: Option<T>) -> Self {
        ParsedValue::Option(value.map(|v| Box::new(v.into())))
    }
}

impl TryFrom<ParsedValue> for Option<String> {
    type Error = ();

    fn try_from(value: ParsedValue) -> Result<Self, Self::Error> {
        if let ParsedValue::Option(opt) = value {
            if let Some(parsed) = opt {
                String::try_from(*parsed).map(Some)
            } else {
                Ok(None)
            }
        } else {
            Err(())
        }
    }
}

impl TryFrom<ParsedValue> for Option<Entity> {
    type Error = ();

    fn try_from(value: ParsedValue) -> Result<Self, Self::Error> {
        if let ParsedValue::Option(opt) = value {
            if let Some(parsed) = opt {
                Entity::try_from(*parsed).map(Some)
            } else {
                Ok(None)
            }
        } else {
            Err(())
        }
    }
}
