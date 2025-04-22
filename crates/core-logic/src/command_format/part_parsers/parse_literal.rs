use crate::command_format::parsed_value::ParsedValue;

use super::{CommandPartParseError, CommandPartParseResult, PartParserContext};

/// Parses a literal value from the provided context.
pub fn parse_literal(literal: &str, context: PartParserContext) -> CommandPartParseResult {
    if let Some(remaining) = context.input.strip_prefix(literal) {
        return CommandPartParseResult::Success {
            parsed: ParsedValue::String(literal.to_string()),
            consumed: literal.to_string(),
            remaining: remaining.to_string(),
        };
    }

    CommandPartParseResult::Failure {
        error: CommandPartParseError::Unmatched { details: None },
        remaining: context.input,
    }
}
