use crate::command_format::parsed_value::ParsedValue;

use super::{
    take_until_literal_if_next, CommandPartParseError, CommandPartParseResult, PartParserContext,
};

/// Parses all the text from the provided context.
/// If the next part to be parsed is a literal, this will stop once that literal is reached.
pub fn parse_any_text(context: PartParserContext) -> CommandPartParseResult {
    let (parsed, remaining) = take_until_literal_if_next(context);

    if parsed.is_empty() {
        return CommandPartParseResult::Failure {
            error: CommandPartParseError::Unmatched { details: None },
            remaining,
        };
    }

    CommandPartParseResult::Success {
        parsed: ParsedValue::String(parsed.clone()),
        consumed: parsed,
        remaining: remaining.to_string(),
    }
}
