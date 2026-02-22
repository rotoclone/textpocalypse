use nonempty::NonEmpty;
use voca_rs::Voca;

use crate::command_format::part_matchers::{
    match_literal, CommandPartMatchError, CommandPartMatchResult, PartMatcherContext,
};

/// Matches a literal value from the provided context.
///
/// As many characters will be matched as possible. For example, if the literals are "123" and "1234" and the input is "12345", then "1234" will be matched.
pub fn match_one_of_literal(
    literals: &NonEmpty<String>,
    context: PartMatcherContext,
) -> CommandPartMatchResult {
    let input = context.input.clone();
    let mut best_match = None;
    for literal in literals {
        if let CommandPartMatchResult::Success { matched, remaining } =
            match_literal(literal, context.clone())
        {
            if matched._count_graphemes()
                > best_match
                    .as_ref()
                    .map(|(m, _): &(String, String)| m._count_graphemes())
                    .unwrap_or(0)
            {
                best_match = Some((matched, remaining));
            }
        }
    }

    if let Some((matched, remaining)) = best_match {
        CommandPartMatchResult::Success { matched, remaining }
    } else {
        CommandPartMatchResult::Failure {
            error: CommandPartMatchError::Unmatched { details: None },
            remaining: input,
        }
    }
}
