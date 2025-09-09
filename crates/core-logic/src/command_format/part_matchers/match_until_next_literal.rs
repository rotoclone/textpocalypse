use crate::command_format::part_matchers::{
    take_until_literal_if_next, CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
};

/// Matches all the text from the provided context.
/// If the next part is a literal, this will stop once that literal is reached.
pub fn match_until_next_literal(context: PartMatcherContext) -> CommandPartMatchResult {
    //TODO allow overriding this behavior by including quotes or something, for targets that confuse the parser
    let (matched, remaining) = take_until_literal_if_next(context);

    if matched.is_empty() {
        return CommandPartMatchResult::Failure {
            error: CommandPartMatchError::Unmatched { details: None },
            remaining,
        };
    }

    CommandPartMatchResult::Success { matched, remaining }
}
