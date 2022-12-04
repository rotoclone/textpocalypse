use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::{
    action::{Action, ActionResult},
    command_parser::{
        Command, CommandError, CommandParseError, CommandParser, CommandTarget, CommandTargetError,
        CommandTargets, CorrectVerbError,
    },
    component::Description,
};

use super::{Connection, ParseCustomCommand};

const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref OPEN_PATTERN: Regex = Regex::new("^open (the )?(?P<name>.*)").unwrap();
    static ref CLOSE_PATTERN: Regex = Regex::new("^close (the )?(?P<name>.*)").unwrap();
}

struct OpenParser {}

impl CommandParser for OpenParser {
    fn parse(&self, input: &str) -> Result<Box<dyn Command>, CommandParseError> {
        if let Some(captures) = OPEN_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                return Ok(Box::new(OpenCommand {
                    target: CommandTarget::parse(target_match.as_str()),
                    should_be_open: true,
                }));
            } else {
                return Err(CommandParseError::CorrectVerb(CorrectVerbError::Target(
                    CommandTargetError::MissingPrimaryTarget,
                )));
            }
        }

        if let Some(captures) = CLOSE_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                return Ok(Box::new(OpenCommand {
                    target: CommandTarget::parse(target_match.as_str()),
                    should_be_open: false,
                }));
            } else {
                return Err(CommandParseError::CorrectVerb(CorrectVerbError::Target(
                    CommandTargetError::MissingPrimaryTarget,
                )));
            }
        }

        Err(CommandParseError::WrongVerb)
    }
}

struct OpenCommand {
    target: CommandTarget,
    should_be_open: bool,
}

impl Command for OpenCommand {
    fn to_action(
        &self,
        commanding_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandError> {
        match &self.target {
            CommandTarget::Myself => Err(CommandError::InvalidPrimaryTarget),
            CommandTarget::Here => Err(CommandError::InvalidPrimaryTarget),
            CommandTarget::Named(target) => {
                if let Some(target) = target.find_target_entity(commanding_entity, world) {
                    Ok(Box::new(OpenAction {
                        target,
                        should_be_open: self.should_be_open,
                    }))
                } else {
                    Err(CommandError::InvalidPrimaryTarget)
                }
            }
        }
    }
}

/// Describes whether an entity is open or closed.
#[derive(Component)]
pub struct OpenState {
    /// Whether the entity is open.
    pub is_open: bool,
}

impl ParseCustomCommand for OpenState {
    fn get_parser() -> Box<dyn CommandParser> {
        Box::new(OpenParser {})
    }
}

#[derive(Debug)]
struct OpenAction {
    target: Entity,
    should_be_open: bool,
}

impl Action for OpenAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let mut state = world
            .get_mut::<OpenState>(self.target)
            .expect("Entity to open or close should have an open state");

        if state.is_open == self.should_be_open {
            if state.is_open {
                return ActionResult::message(
                    performing_entity,
                    "It's already open.".to_string(),
                    false,
                );
            } else {
                return ActionResult::message(
                    performing_entity,
                    "It's already closed.".to_string(),
                    false,
                );
            }
        }

        // if trying to open and entity is locked and can be unlocked, unlock it first
        //TODO

        state.is_open = self.should_be_open;
        set_other_side_open(self.target, self.should_be_open, world);

        let name = world
            .get::<Description>(self.target)
            .map_or("it".to_string(), |n| format!("the {}", n.name));

        if self.should_be_open {
            ActionResult::message(performing_entity, format!("You open {name}."), true)
        } else {
            ActionResult::message(performing_entity, format!("You close {name}."), true)
        }
    }
}

/// Sets the other side of this entity to the provided open state, if it has one.
fn set_other_side_open(this_side: Entity, should_be_open: bool, world: &mut World) {
    if let Some(other_side_id) = world
        .get::<Connection>(this_side)
        .and_then(|c| c.other_side)
    {
        if let Some(mut other_side_state) = world.get_mut::<OpenState>(other_side_id) {
            other_side_state.is_open = should_be_open;
        }
    }

    //TODO send messages to entities on the other side of the entity telling them it closed
}
