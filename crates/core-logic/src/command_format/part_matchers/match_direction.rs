use crate::{
    command_format::{
        part_matchers::{
            take_until, CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
        },
        DirectionMatchMode,
    },
    Direction,
};

/// Matches a direction from the provided context.
pub fn match_direction(
    match_mode: DirectionMatchMode,
    context: PartMatcherContext,
) -> CommandPartMatchResult {
    // all directions are one word, so take until a space or the end of the input
    let (matched, remaining) = take_until(context.input, Some(" "));
    if matched.is_empty() {
        return CommandPartMatchResult::Failure {
            error: CommandPartMatchError::Unmatched { details: None },
            remaining,
        };
    }

    match match_mode {
        DirectionMatchMode::Anything => CommandPartMatchResult::Success { matched, remaining },
        DirectionMatchMode::OnlyValidDirections => {
            // this is necessary because otherwise the command format that's just a direction would match on every possible command
            // and so every input would fail with "that's not a direction"
            if Direction::parse(&matched).is_some() {
                CommandPartMatchResult::Success { matched, remaining }
            } else {
                CommandPartMatchResult::Failure {
                    error: CommandPartMatchError::Unmatched { details: None },
                    // put parts back together
                    remaining: format!("{matched}{remaining}"),
                }
            }
        }
    }
}
