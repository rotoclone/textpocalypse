use nom::IResult;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Command {
    verb: Verb,
    primary_target: Option<CommandTarget>,
    secondary_target: Option<CommandTarget>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Verb {
    Look,
    Move,
    Get,
    Put,
    Open,
    Close,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CommandTarget {
    name: String,                // TODO this should probably be an entity
    location_chain: Vec<String>, // TODO this should probably also be an entity
}

pub fn parse_input(input: &str) -> IResult<&str, Command> {
    todo!() //TODO
}

fn verb(input: &str) -> IResult<&str, Verb> {
    todo!() //TODO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_verb() {
        let expected = Command {
            verb: Verb::Look,
            primary_target: None,
            secondary_target: None,
        };

        assert_eq!(Ok(("", expected)), parse_input("look"));
    }

    #[test]
    fn single_subject() {
        let expected = Command {
            verb: Verb::Look,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: Vec::new(),
            }),
            secondary_target: None,
        };

        assert_for_inputs(expected, &["l thing", "look thing", "look at thing"]);
    }

    #[test]
    fn multiple_subjects() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: Vec::new(),
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: Vec::new(),
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in other thing", "put thing into other thing"],
        );
    }

    #[test]
    fn multiple_subjects_with_locations() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec!["place".to_string()],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: vec!["other place".to_string()],
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in place into other thing in other place"],
        );
    }

    #[test]
    fn multiple_subjects_with_multiple_locations() {
        let expected = Command {
            verb: Verb::Put,
            primary_target: Some(CommandTarget {
                name: "thing".to_string(),
                location_chain: vec!["other place".to_string(), "place".to_string()],
            }),
            secondary_target: Some(CommandTarget {
                name: "other thing".to_string(),
                location_chain: vec!["yet another place".to_string(), "another place".to_string()],
            }),
        };

        assert_for_inputs(
            expected,
            &["put thing in place in other place into other thing in another place in yet another place"],
        );
    }

    fn assert_for_inputs(expected: Command, inputs: &[&str]) {
        for input in inputs {
            assert_eq!(Ok(("", expected.clone())), parse_input(input));
        }
    }
}
