use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    input_parser::{InputParseError, InputParser},
    GameMessage, HelpMessage, World,
};

use super::{Action, ActionResult};

const HELP_FORMAT: &str = "help";

lazy_static! {
    static ref HELP_PATTERN: Regex = Regex::new("^help$").unwrap();
}

pub struct HelpParser;

impl InputParser for HelpParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if HELP_PATTERN.is_match(input) {
            return Ok(Box::new(HelpAction));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![HELP_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
struct HelpAction;

impl Action for HelpAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let message = GameMessage::Help(HelpMessage::for_entity(performing_entity, world));

        ActionResult::builder_no_tick()
            .with_game_message(performing_entity, message)
            .build()
    }
}
