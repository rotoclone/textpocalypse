use crate::{command_format::parsed_value::ParsedValue, Direction};

use super::{
    take_until_literal_if_next, CommandPartParseError, CommandPartParseResult, PartParserContext,
};

/// Parses a direction from the provided context.
pub fn parse_direction(context: PartParserContext) -> CommandPartParseResult {
    let (to_parse, remaining) = take_until_literal_if_next(context);

    if let Some(direction) = Direction::parse(&to_parse) {
        CommandPartParseResult::Success {
            parsed: ParsedValue::Direction(direction),
            consumed: to_parse,
            remaining,
        }
    } else {
        CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched {
                details: Some(format!("'{to_parse}' is not a direction.")),
            },
            // re-combine here to avoid an extra clone above in non-error cases
            remaining: format!("{to_parse}{remaining}"),
        }
    }
}
