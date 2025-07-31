use crate::command_format::part_matchers::{
    CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
};

/// Matches a literal value from the provided context.
pub fn match_literal(literal: &str, context: PartMatcherContext) -> CommandPartMatchResult {
    if let Some(remaining) = context.input.strip_prefix(literal) {
        return CommandPartMatchResult::Success {
            matched: literal.to_string(),
            remaining: remaining.to_string(),
        };
    }

    CommandPartMatchResult::Failure {
        error: CommandPartMatchError::Unmatched { details: None },
        remaining: context.input,
    }
}
