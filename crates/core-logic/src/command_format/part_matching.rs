use crate::command_format::CommandFormatPart;

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
struct MatchedCommand {
    //TODO
}
