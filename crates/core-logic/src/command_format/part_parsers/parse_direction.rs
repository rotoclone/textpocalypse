use crate::{command_format::parsed_value::ParsedValue, Direction};

use super::{CommandPartParseError, CommandPartParseResult, PartParserContext};

/// Parses a direction from the provided context.
pub fn parse_direction(context: PartParserContext) -> CommandPartParseResult {
    if let Some(direction) = Direction::parse(&context.input) {
        CommandPartParseResult::Success(ParsedValue::Direction(direction))
    } else {
        CommandPartParseResult::Failure(CommandPartParseError::Unparseable {
            details: Some(format!("'{}' is not a direction.", context.input)),
        })
    }
}
