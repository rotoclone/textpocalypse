use crate::command_format::part_matchers::{
    take_until, CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
};

/// Matches a direction from the provided context.
pub fn match_direction(context: PartMatcherContext) -> CommandPartMatchResult {
    // all directions are one word, so take until a space or the end of the input
    let (matched, remaining) = take_until(context.input, Some(" "));
    if matched.is_empty() {
        return CommandPartMatchResult::Failure {
            error: CommandPartMatchError::Unmatched { details: None },
            remaining,
        };
    }

    CommandPartMatchResult::Success { matched, remaining }
}
