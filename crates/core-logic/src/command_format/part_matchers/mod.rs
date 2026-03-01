use std::collections::HashMap;
use std::collections::HashSet;

use bevy_ecs::prelude::*;
use log::warn;
use nonempty::nonempty;
use nonempty::NonEmpty;
use voca_rs::Voca;

use crate::command_format::{
    part_parsers::CommandPartParseResult, CommandFormat, CommandFormatPart,
    ParsedCommandFormatPart, PartParserContext, UntypedCommandPartId,
};

mod match_literal;
pub use match_literal::match_literal;

mod match_one_of_literal;
pub use match_one_of_literal::match_one_of_literal;

mod match_until_next_literal;
pub use match_until_next_literal::match_until_next_literal;

mod match_direction;
pub use match_direction::match_direction;

/// Context included when matching input to parts
#[derive(Clone)]
pub struct PartMatcherContext<'c> {
    /// The input to match
    pub input: String,
    /// The next parts in the command format, in order
    pub next_parts: Vec<&'c CommandFormatPart>,
}

/// The result of attempting to match input to a part
#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartMatchResult {
    /// The part matched some input (or didn't but it's fine, in which case `matched` will be an empty string)
    Success { matched: String, remaining: String },
    /// The part didn't match any input
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedCommandFormatPart {
    /// The index of this part in the list of parts in the format
    pub order: usize,
    /// The part that was matched
    pub part: CommandFormatPart,
    /// The portion of the input that was determined to correspond with this part
    pub matched_input: String,
}

impl MatchedCommandFormatPart {
    /// Parses this matched part into an actual parsed value.
    pub fn parse(
        &self,
        entering_entity: Entity,
        parsed_parts: HashMap<UntypedCommandPartId, ParsedCommandFormatPart>,
        world: &World,
    ) -> CommandPartParseResult {
        self.part.parse(
            PartParserContext {
                input: self.matched_input.clone(),
                entering_entity,
                parsed_parts,
            },
            world,
        )
    }
}

/// An intermediate state during command parsing, where some parts may have been associated with a portion of the input string, but the parts haven't actually been parsed yet.
pub struct MatchedCommand {
    /// The parts that were successfully matched
    pub matched_parts: Vec<MatchedCommandFormatPart>,
    /// Any parts that weren't matched
    pub unmatched_parts: Vec<CommandFormatPart>,
    /// Any remaining un-matched input
    pub remaining_input: String,
}

impl MatchedCommand {
    /// Attempts to match parts from a format to portions of the provided input.
    pub fn from_format(format: &CommandFormat, input: impl Into<String>) -> MatchedCommand {
        let mut remaining_input = input.into();
        let mut matched_parts = Vec::new();

        for (i, part) in format.0.iter().enumerate() {
            match part.match_from(PartMatcherContext {
                input: remaining_input,
                next_parts: format.0.iter().skip(i + 1).collect(),
            }) {
                CommandPartMatchResult::Success { matched, remaining } => {
                    matched_parts.push(MatchedCommandFormatPart {
                        order: i,
                        part: part.clone(),
                        matched_input: matched,
                    });

                    remaining_input = remaining;
                }
                CommandPartMatchResult::Failure { remaining, .. } => {
                    let unmatched_parts =
                        format.0.iter().skip(matched_parts.len()).cloned().collect();
                    return MatchedCommand {
                        matched_parts,
                        unmatched_parts,
                        remaining_input: remaining,
                    };
                }
            }
        }

        MatchedCommand {
            matched_parts,
            unmatched_parts: Vec::new(),
            remaining_input,
        }
    }
}

enum LiteralPart<'s> {
    Single(&'s String),
    Optional(&'s String),
    OneOf(&'s NonEmpty<String>),
}

/// If the next parts are one or more consecutive `Literal`, `OptionalLiteral`, and/or `OneOfLiteral` parts: returns a tuple of the input up until the literal(s), and the input including and after the literal(s).
///
/// Otherwise: returns `(input, "")`.
pub fn take_until_literal_if_next(context: PartMatcherContext) -> (String, String) {
    let mut next_literal_parts = Vec::new();
    for next_part in &context.next_parts {
        match next_part {
            CommandFormatPart::Literal(s, _) => next_literal_parts.push(LiteralPart::Single(s)),
            CommandFormatPart::OptionalLiteral(s, _) => {
                next_literal_parts.push(LiteralPart::Optional(s))
            }
            CommandFormatPart::OneOfLiteral(literals, _) => {
                next_literal_parts.push(LiteralPart::OneOf(literals))
            }
            _ => break,
        };
    }

    let permutations = generate_literal_permutations(&next_literal_parts)
        .into_iter()
        .collect::<HashSet<String>>();
    if permutations.len() > 15 {
        warn!(
            "Format generated a large number ({}) of literal permutations. Parts: {:?}",
            permutations.len(),
            &context.next_parts
        );
    }
    let mut best_match: Option<(String, String)> = None;

    //TODO this requires fully matching the permutations, so if it's looking for " from " then "thing from" (no trailing space) won't match anything and ("thing from", "") will be returned, but in that case it should probably return ("thing", " from")
    for permutation in &permutations {
        let (taken, remaining) = take_until(&context.input, Some(permutation));
        // if `remaining` is empty that means the permutation wasn't in the input, so without this check if the permutation is never found `best_match` will be `Some` and the partial match fallback will never be hit
        if !remaining.is_empty() {
            if let Some((best_taken, _)) = &best_match {
                // "best" is considered the smallest amount of characters consumed, i.e. the first instance of the literal(s)
                if taken._count_graphemes() < best_taken._count_graphemes() {
                    best_match = Some((taken, remaining));
                }
            } else {
                best_match = Some((taken, remaining));
            }
        }
    }

    if let Some((matched, remaining)) = best_match {
        (matched, remaining)
    } else {
        // there aren't any good matches, so parsing is going to fail anyway, but check if there are any partial matches so the error message is better
        if let Some((matched, remaining)) = find_best_partial_match(&context.input, &permutations) {
            (matched, remaining)
        } else {
            (context.input, "".to_string())
        }
    }
}

/// Generates all the valid permutations of the provided literal parts.
/// For example, if two `OneOf` parts are provided, one with "a" or "b" and one with "c" or "d", then ["ac", "ad", "bc", "bd"] will be returned.
fn generate_literal_permutations(next_literal_parts: &[LiteralPart]) -> Vec<String> {
    // base case
    if next_literal_parts.is_empty() {
        return vec!["".to_string()];
    }

    // generate permutations for all but the last part
    let mut permutations =
        generate_literal_permutations(&next_literal_parts[..next_literal_parts.len() - 1]);

    // now add the permutation(s) for the last part
    if let Some(last_part) = next_literal_parts.last() {
        let strs_to_append = match last_part {
            LiteralPart::Single(s) => NonEmpty::new(s.as_str()),
            LiteralPart::Optional(s) => nonempty![s.as_str(), ""],
            LiteralPart::OneOf(literals) => {
                let mut strs = NonEmpty::new(literals.first().as_str());
                strs.extend(literals.iter().map(String::as_str));
                strs
            }
        };

        let mut new_permutations = HashSet::new();
        for permutation in permutations.iter_mut() {
            for (i, to_append) in strs_to_append.iter().enumerate() {
                if i == strs_to_append.len() - 1 {
                    // this is the last one, so the existing permutation can be modified in-place
                    *permutation += to_append;
                } else {
                    // this is the not-last of more than one string to append, so a new permutation needs to be created
                    new_permutations.insert(permutation.clone() + to_append);
                }
            }
        }
        permutations.extend(new_permutations);
    }

    permutations
}

/// Finds the best match among the provided literal permutations, given that `input` doesn't actually contain any of the permutations, returning `(matched, remaining)`.
/// Returns `None` if none of the permutations are matched at all.
///
/// For example, if `input` is `"the thing 1"` and `next_literal_permutations` is `[" 123", " 456"]` then this will return `Some(("the thing", " 1"))`.
/// If `input` was `"the thing 2"` or `"the thing 13"` then `None` would be returned.
fn find_best_partial_match(
    input: &str,
    next_literal_permutations: &HashSet<String>,
) -> Option<(String, String)> {
    let mut best_split_idx = None;
    for permutation in next_literal_permutations {
        let starting_indices = input._index_all(&permutation._grapheme_at(0), 0);
        'starting_idx: for starting_index in starting_indices {
            let mut input_index = starting_index;
            for permutation_char in permutation._graphemes() {
                if permutation_char != input._grapheme_at(input_index) {
                    // this ain't it
                    continue 'starting_idx;
                }

                if input_index == input._count_graphemes() - 1 {
                    // made it to the end of the input without a mismatched character
                    if let Some(best_idx) = best_split_idx {
                        // lower index means more of the permutation matched
                        if starting_index < best_idx {
                            best_split_idx = Some(starting_index);
                        }
                    } else {
                        best_split_idx = Some(starting_index);
                    }
                }

                input_index += 1;
            }
        }
    }

    if let Some(split_idx) = best_split_idx {
        let input_graphemes = input._graphemes();
        let (matched, remaining) = input_graphemes.split_at(split_idx);
        Some((matched.join(""), remaining.join("")))
    } else {
        None
    }
}

/// Splits `input` at the first instance of `stopping_point`, returning a tuple of the input before `stopping_point`, and the input including and after `stopping_point`.
/// If `stopping_point` is `None`, an empty string, or isn't in `input`, returns `(input, "")`.
pub fn take_until(input: impl Into<String>, stopping_point: Option<&str>) -> (String, String) {
    let input = input.into();
    if let Some(stopping_point) = stopping_point {
        if stopping_point.is_empty() || !input.contains(stopping_point) {
            // `_before` returns an empty string if the provided substring isn't found, but for the purposes of this function we want the whole input in that case
            return (input.clone(), "".to_string());
        }

        let parsed = if input.starts_with(stopping_point) {
            // `_before` doesn't properly handle if the string starts with the provided substring, so deal with that case manually
            // this check can be removed once a new version of voca_rs is released that includes the changes from https://github.com/a-merezhanyi/voca_rs/pull/27
            "".to_string()
        } else {
            input._before(stopping_point)
        };
        let remaining = input.strip_prefix(&parsed).unwrap_or_default();
        (parsed, remaining.to_string())
    } else {
        (input.clone(), "".to_string())
    }
}

/// Converts `CommandPartMatchResult::Failure` to `CommandPartMatchResult::Success` with a matched value of `None`.
/// Doesn't touch `CommandPartMatchResult::Success`.
pub fn match_result_to_option(match_result: CommandPartMatchResult) -> CommandPartMatchResult {
    match match_result {
        CommandPartMatchResult::Success { matched, remaining } => {
            CommandPartMatchResult::Success { matched, remaining }
        }
        CommandPartMatchResult::Failure { remaining, .. } => CommandPartMatchResult::Success {
            matched: String::new(),
            remaining,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::command_format::{entity_part, literal_part, one_of_literal_part, CommandPartId};

    use super::*;

    #[test]
    fn take_until_empty_input_no_stopping_point() {
        assert_eq!(("".to_string(), "".to_string()), take_until("", None));
    }

    #[test]
    fn take_until_no_stopping_point() {
        assert_eq!(
            ("some input".to_string(), "".to_string()),
            take_until("some input", None)
        );
    }

    #[test]
    fn take_until_empty_stopping_point() {
        assert_eq!(
            ("some input".to_string(), "".to_string()),
            take_until("some input", Some(""))
        );
    }

    #[test]
    fn take_until_stopping_point_not_in_input() {
        assert_eq!(
            ("some input".to_string(), "".to_string()),
            take_until("some input", Some("hello"))
        );
    }

    #[test]
    fn take_until_stopping_point_at_beginning_of_input() {
        assert_eq!(
            ("".to_string(), "some input".to_string()),
            take_until("some input", Some("so"))
        );
    }

    #[test]
    fn take_until_stopping_point_at_end_of_input() {
        assert_eq!(
            ("some in".to_string(), "put".to_string()),
            take_until("some input", Some("put"))
        );
    }

    #[test]
    fn take_until_stopping_point_in_middle_of_input() {
        assert_eq!(
            ("som".to_string(), "e input".to_string()),
            take_until("some input", Some("e in"))
        );
    }

    #[test]
    fn take_until_literal_if_next_no_next_parts() {
        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: Vec::new()
            })
        );

        assert_eq!(
            ("hello".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: Vec::new()
            })
        );

        assert_eq!(
            ("hello there".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello there".to_string(),
                next_parts: Vec::new()
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_non_literal_part_next() {
        let part_1 = entity_part(CommandPartId::new("id"));
        let part_2 = literal_part("hello");
        let next_parts = vec![&part_1, &part_2];

        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hi".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hi".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("he".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hello".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hello there".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_single_literal_part_next() {
        let next_part = literal_part("hello");
        let next_parts = vec![&next_part];

        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hi".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hi".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why he do dat".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he do dat".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hellohello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hellohello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_single_literal_part_then_non_literal_part_next() {
        let part_1 = literal_part("hello");
        let part_2 = entity_part(CommandPartId::new("id"));
        let next_parts = vec![&part_1, &part_2];

        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hi".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hi".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why he do dat".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he do dat".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hellohello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hellohello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_multiple_literal_parts_next() {
        let part_1 = literal_part("hello");
        let part_2 = literal_part(" there");
        let next_parts = vec![&part_1, &part_2];

        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hi".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hi".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why he do dat".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why he do dat".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hello ".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hello".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hellohello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why hello ".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why hello goodbye".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello goodbye".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("there".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            (" there".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: " there".to_string(),
                next_parts: next_parts.clone(),
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_single_one_of_literal_part_next() {
        let next_part = one_of_literal_part(nonempty!["hello", "hello there", "bye"]);
        let next_parts = vec![&next_part];

        assert_eq!(
            ("".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("hi".to_string(), "".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hi".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "he".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "he".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "hello there aaah".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why hello there aaah".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "by".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "by".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "bye".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "bye".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("why ".to_string(), "bye there".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "why bye there".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "bye hello".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "bye hello".to_string(),
                next_parts: next_parts.clone(),
            })
        );

        assert_eq!(
            ("".to_string(), "hello bye".to_string()),
            take_until_literal_if_next(PartMatcherContext {
                input: "hello bye".to_string(),
                next_parts: next_parts.clone(),
            })
        );
    }

    #[test]
    fn take_until_literal_if_next_single_optional_literal_part_next() {
        //TODO
    }

    #[test]
    fn take_until_literal_if_next_multiple_assorted_literal_parts_next() {
        //TODO
    }
}
