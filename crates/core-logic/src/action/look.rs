use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    component::{Description, Room},
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
    EntityDescription, GameMessage, RoomDescription, World,
};

use super::{Action, ActionResult};

const LOOK_VERB_NAME: &str = "look";
const LOOK_TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref LOOK_PATTERN: Regex = Regex::new("^l(ook)?( (at )?(the )?(?P<target>.*))?").unwrap();
}

pub struct LookParser;

impl InputParser for LookParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = LOOK_PATTERN.captures(input) {
            // looking
            if let Some(target_match) = captures.name(LOOK_TARGET_CAPTURE) {
                // looking at something specific
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    // looking at something they can see
                    return Ok(Box::new(LookAction {
                        target: target_entity,
                    }));
                } else {
                    return Err(InputParseError::CommandParseError {
                        verb: LOOK_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            } else {
                // just looking in general
                if let Some(target) = CommandTarget::Here.find_target_entity(source_entity, world) {
                    return Ok(Box::new(LookAction { target }));
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }
}

#[derive(Debug)]
struct LookAction {
    target: Entity,
}

impl Action for LookAction {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, performing_entity) {
            return ActionResult::none();
        }

        let target = world.entity(self.target);

        if let Some(room) = target.get::<Room>() {
            return ActionResult {
                messages: [(
                    performing_entity,
                    vec![GameMessage::Room(RoomDescription::from_room(
                        room,
                        performing_entity,
                        world,
                    ))],
                )]
                .into(),
                should_tick: false,
            };
        }

        if let Some(desc) = target.get::<Description>() {
            return ActionResult {
                messages: [(
                    performing_entity,
                    vec![GameMessage::Entity(EntityDescription::from_description(
                        desc,
                    ))],
                )]
                .into(),
                should_tick: false,
            };
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }
}
