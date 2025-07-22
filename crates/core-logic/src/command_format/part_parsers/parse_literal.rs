use crate::command_format::parsed_value::ParsedValue;

use super::{CommandPartParseResult, PartParserContext};

/// Parses a literal value from the provided context.
/// Will always succeed, since no conversion is necessary.
pub fn parse_literal(context: PartParserContext) -> CommandPartParseResult {
    CommandPartParseResult::Success(ParsedValue::String(context.input))
}
