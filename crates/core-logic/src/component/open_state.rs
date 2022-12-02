use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::{
    action::{Action, ActionResult},
    component::Description,
};

use super::{Connection, ParseCommand};

const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref OPEN_PATTERN: Regex = Regex::new("^open (the )?(?P<name>.*)").unwrap();
    static ref CLOSE_PATTERN: Regex = Regex::new("^close (the )?(?P<name>.*)").unwrap();
}

/// Describes whether an entity is open or closed.
#[derive(Component)]
pub struct OpenState {
    /// Whether the entity is open.
    pub is_open: bool,
}

impl ParseCommand for OpenState {
    fn parse_command(
        this_entity_id: Entity,
        input: &str,
        commanding_entity_id: Entity,
        world: &World,
    ) -> Option<Box<dyn Action>> {
        debug!("Entity {this_entity_id:?} parsing command {input:?} from {commanding_entity_id:?}");

        if let Some(desc) = world.get::<Description>(this_entity_id) {
            // opening
            if let Some(captures) = OPEN_PATTERN.captures(input) {
                if let Some(target_match) = captures.name(NAME_CAPTURE) {
                    if desc.matches(target_match.as_str()) {
                        let action = OpenAction {
                            target: this_entity_id,
                            should_be_open: true,
                        };
                        return Some(Box::new(action));
                    }
                }
            }

            // closing
            if let Some(captures) = CLOSE_PATTERN.captures(input) {
                if let Some(target_match) = captures.name(NAME_CAPTURE) {
                    if desc.matches(target_match.as_str()) {
                        let action = OpenAction {
                            target: this_entity_id,
                            should_be_open: false,
                        };
                        return Some(Box::new(action));
                    }
                }
            }
        }

        None
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
                return ActionResult::message(performing_entity, "It's already open.".to_string());
            } else {
                return ActionResult::message(
                    performing_entity,
                    "It's already closed.".to_string(),
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

        // TODO set should_tick to true
        if self.should_be_open {
            ActionResult::message(performing_entity, format!("You open {name}."))
        } else {
            ActionResult::message(performing_entity, format!("You close {name}."))
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
