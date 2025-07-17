use nonempty::NonEmpty;

use crate::command_format::{CommandFormat, CommandFormatParseError, CommandFormatPart};

/// TODO doc
#[derive(Clone)]
pub struct PartMatcherContext<'c> {
    pub input: String,
    pub next_part: Option<&'c CommandFormatPart>,
}

//TODO doc
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartMatchResult {
    Success {
        matched: String,
        remaining: String,
    },
    Failure {
        error: CommandPartMatchError,
        remaining: String,
    },
}

/// An error encountered while attempting to match a command part.
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartMatchError {
    /// All the input was consumed before getting to this part
    EndOfInput,
    /// The part was not matched
    Unmatched { details: Option<String> },
}

/// A part that has been associated with a portion of the input string
#[derive(Debug)]
pub struct MatchedCommandFormatPart {
    pub part: CommandFormatPart,
    pub matched_input: String,
}

/// An intermediate state during command parsing, where each part has been associated with a portion of the input string, but the parts haven't actually been parsed yet.
struct MatchedCommand(NonEmpty<MatchedCommandFormatPart>);

/// Attempts to match parts from a format to portions of the provided input.
/// TODO move to part_matching.rs?
pub fn match_parts(
    format: CommandFormat,
    input: impl Into<String>,
) -> Result<MatchedCommand, CommandFormatParseError> {
    let mut remaining_input = input.into();
    let mut has_remaining_input = true;
    let mut matched_parts = Vec::new();

    for (i, part) in format.0.iter().enumerate() {
        if remaining_input.is_empty() {
            has_remaining_input = false;
        }

        match part.match_from(PartMatcherContext {
            input: remaining_input,
            next_part: format.0.get(i + 1),
        }) {
            CommandPartMatchResult::Success { matched, remaining } => {
                matched_parts.push(MatchedCommandFormatPart {
                    part: part.clone(),
                    matched_input: matched,
                });

                remaining_input = remaining;
            }
            CommandPartMatchResult::Failure { error, .. } => {
                let mut unmatched_parts = NonEmpty::new(part.clone());
                // +1 to account for the failed part already added above
                unmatched_parts.extend(format.0.iter().skip(matched_parts.len() + 1).cloned());

                if !has_remaining_input {
                    // Assume that this part failed to match due to the input being empty. This has to be down here because some parts
                    // may be optional, in which case they will match just fine with no input, so this shouldn't pre-emptively return
                    // an end of input error without letting the part see if that's actually a problem first.
                    //TODO is it a problem to just throw away the error returned from the part?
                    return Err(CommandFormatParseError::Matching {
                        matched_parts,
                        unmatched_parts: Box::new(unmatched_parts),
                        error: CommandPartMatchError::EndOfInput,
                    });
                }

                return Err(CommandFormatParseError::Matching {
                    matched_parts,
                    unmatched_parts: Box::new(unmatched_parts),
                    error,
                });
            }
        }
    }

    if !remaining_input.is_empty() {
        return Err(CommandFormatParseError::UnmatchedInput {
            matched_parts,
            unmatched: remaining_input,
        });
    }

    Ok(MatchedCommand(
        // the format's parts are `NonEmpty`; each one that successfully matches gets added to `matched_parts`; if any don't successfully match an error is returned before this point
        NonEmpty::collect(matched_parts).expect("matched parts should not be empty"),
    ))
}
