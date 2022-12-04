use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    can_receive_messages,
    command_parser::{
        Command, CommandError, CommandParseError, CommandParser, CommandTarget, CommandTargets,
    },
    component::{Description, Room},
    EntityDescription, GameMessage, RoomDescription, World,
};

use super::{Action, ActionResult};

const LOOK_TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref LOOK_PATTERN: Regex = Regex::new("^l(ook)?( (at )?(the )?(?P<target>.*))?").unwrap();
}

pub struct LookParser;

impl CommandParser for LookParser {
    fn parse(&self, input: &str) -> Result<Box<dyn Command>, CommandParseError> {
        if let Some(captures) = LOOK_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(LOOK_TARGET_CAPTURE) {
                return Ok(Box::new(LookCommand {
                    targets: CommandTargets {
                        primary: Some(CommandTarget::parse(target_match.as_str())),
                        secondary: None,
                    },
                }));
            } else {
                return Ok(Box::new(LookCommand {
                    targets: CommandTargets {
                        primary: Some(CommandTarget::Here),
                        secondary: None,
                    },
                }));
            }
        }

        Err(CommandParseError::WrongVerb)
    }
}

struct LookCommand {
    targets: CommandTargets,
}

impl Command for LookCommand {
    fn to_action(
        &self,
        commanding_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandError> {
        if let Some(target) = &self.targets.primary {
            if let Some(target_entity) = target.find_target_entity(commanding_entity, world) {
                return Ok(Box::new(Look {
                    target: target_entity,
                }));
            }
        }

        Err(CommandError::InvalidPrimaryTarget)
    }
}

#[derive(Debug)]
struct Look {
    target: Entity,
}

impl Action for Look {
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
