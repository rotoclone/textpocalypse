use bevy_ecs::prelude::*;

use crate::command_format::CommandFormatPart;

use super::{CommandPartParseError, CommandPartParseResult, PartParserContext};

/// Finds the first part in `parts` that successfully parses.
/// If none of the parts parse successfully, returns the first error encountered.
pub fn parse_one_of(
    parts: &[CommandFormatPart],
    context: PartParserContext,
    world: &World,
) -> CommandPartParseResult {
    let mut first_error = None;
    for part in parts {
        match part.parse(context.clone(), world) {
            CommandPartParseResult::Success {
                parsed,
                consumed,
                remaining,
            } => {
                return CommandPartParseResult::Success {
                    parsed,
                    consumed,
                    remaining,
                };
            }
            CommandPartParseResult::Failure { error, .. } => {
                first_error.get_or_insert(error);
            }
        }
    }

    CommandPartParseResult::Failure {
        error: first_error.unwrap_or(CommandPartParseError::Unmatched { details: None }),
        remaining: context.input,
    }
}
