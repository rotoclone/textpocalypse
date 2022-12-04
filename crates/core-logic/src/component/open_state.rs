use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{Action, ActionResult},
    component::Description,
    get_reference_name,
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
};

use super::{Connection, ParseCustomInput};

const SLAM_VERB_NAME: &str = "slam";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref SLAM_PATTERN: Regex = Regex::new("^slam (the )?(?P<name>.*)").unwrap();
}

struct SlamParser;

impl InputParser for SlamParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = SLAM_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let command_target = CommandTarget::parse(target_match.as_str());
                if let Some(target) = command_target.find_target_entity(source_entity, world) {
                    return Ok(Box::new(SlamAction { target }));
                } else {
                    return Err(InputParseError::CommandParseError {
                        verb: SLAM_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(command_target),
                    });
                }
            } else {
                return Err(InputParseError::CommandParseError {
                    verb: SLAM_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }
}

#[derive(Debug)]
struct SlamAction {
    target: Entity,
}

impl Action for SlamAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let state = match world.get::<OpenState>(self.target) {
            Some(s) => s,
            None => {
                return ActionResult::error(performing_entity, "You can't slam that.".to_string());
            }
        };

        if !state.is_open {
            return ActionResult::message(
                performing_entity,
                "It's already closed.".to_string(),
                false,
            );
        }

        OpenState::set_open(self.target, false, world);

        let name = get_reference_name(self.target, world);
        ActionResult::message(
            performing_entity,
            format!("You SLAM {name} with a loud bang. You hope you didn't wake up the neighbors."),
            true,
        )
    }
}

/// Describes whether an entity is open or closed.
#[derive(Component)]
pub struct OpenState {
    /// Whether the entity is open.
    pub is_open: bool,
}

impl OpenState {
    /// Sets the open state of the provided entity.
    pub fn set_open(entity: Entity, should_be_open: bool, world: &mut World) {
        // this side
        if let Some(mut state) = world.get_mut::<OpenState>(entity) {
            state.is_open = should_be_open;
        }

        // other side
        if let Some(other_side_id) = world.get::<Connection>(entity).and_then(|c| c.other_side) {
            if let Some(mut other_side_state) = world.get_mut::<OpenState>(other_side_id) {
                other_side_state.is_open = should_be_open;
                //TODO send messages to entities on the other side of the entity telling them it opened or closed
            }
        }
    }
}

impl ParseCustomInput for OpenState {
    fn get_parser() -> Box<dyn InputParser> {
        Box::new(SlamParser)
    }
}
