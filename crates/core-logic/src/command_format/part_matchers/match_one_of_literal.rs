use nonempty::NonEmpty;

use crate::command_format::part_matchers::{
    match_literal, CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
};

/// Matches a literal value from the provided context.
pub fn match_one_of_literal(
    literals: &NonEmpty<String>,
    context: PartMatcherContext,
) -> CommandPartMatchResult {
    let input = context.input.clone();
    for literal in literals {
        if let CommandPartMatchResult::Success { matched, remaining } =
            match_literal(literal, context.clone())
        {
            return CommandPartMatchResult::Success { matched, remaining };
        }
    }

    CommandPartMatchResult::Failure {
        error: CommandPartMatchError::Unmatched { details: None },
        remaining: input,
    }
}
