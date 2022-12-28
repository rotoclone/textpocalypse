use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{
        Action, ActionInterruptResult, ActionNotificationSender, ActionResult, MoveAction,
        OpenAction,
    },
    get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::{Notification, VerifyResult},
    BeforeActionNotification, GameMessage, MessageDelay, VerifyActionNotification,
};

use super::{
    description::DescribeAttributes, queue_action_first, AfterActionNotification,
    AttributeDescriber, AttributeDescription, AttributeDetailLevel, Connection, Container,
    Description, Location, ParseCustomInput,
};

const SLAM_VERB_NAME: &str = "slam";
const SLAM_FORMAT: &str = "slam <>";
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
                    return Ok(Box::new(SlamAction {
                        target,
                        notification_sender: ActionNotificationSender::new(),
                    }));
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

    fn get_input_formats(&self) -> Vec<String> {
        vec![SLAM_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<OpenState>(entity, world, &[SLAM_FORMAT])
    }
}

#[derive(Debug)]
struct SlamAction {
    target: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for SlamAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
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
                MessageDelay::Short,
                false,
            );
        }

        OpenState::set_open(self.target, false, world);

        let name = get_reference_name(self.target, world);
        ActionResult::message(
            performing_entity,
            format!("You SLAM {name} with a loud bang. You hope you didn't wake up the neighbors."),
            MessageDelay::Long,
            true,
        )
    }

    fn interrupt(&self, performing_entity: Entity, _: &World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop slamming.".to_string(),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
    }

    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_before_notification(notification_type, self, world);
    }

    fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        world: &mut World,
    ) -> VerifyResult {
        self.notification_sender
            .send_verify_notification(notification_type, self, world)
    }

    fn send_after_notification(
        &self,
        notification_type: AfterActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_notification(notification_type, self, world);
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

/// Describes whether the entity is open or not.
#[derive(Debug)]
struct OpenStateAttributeDescriber;

impl AttributeDescriber for OpenStateAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(open_state) = world.get::<OpenState>(entity) {
            let description = if open_state.is_open { "open" } else { "closed" };

            return vec![AttributeDescription::is(description.to_string())];
        }

        Vec::new()
    }
}

impl DescribeAttributes for OpenState {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(OpenStateAttributeDescriber)
    }
}

/// Attempts to open openable entities automatically before an attempt is made to move through a closed one.
pub fn auto_open_connections(
    notification: &Notification<BeforeActionNotification, MoveAction>,
    world: &mut World,
) {
    if let Some(current_location) =
        world.get::<Location>(notification.notification_type.performing_entity)
    {
        if let Some(location) = world.get::<Container>(current_location.id) {
            if let Some((connecting_entity, _)) =
                location.get_connection_in_direction(&notification.contents.direction, world)
            {
                if let Some(open_state) = world.get::<OpenState>(connecting_entity) {
                    if !open_state.is_open {
                        queue_action_first(
                            world,
                            notification.notification_type.performing_entity,
                            Box::new(OpenAction {
                                target: connecting_entity,
                                should_be_open: true,
                                notification_sender: ActionNotificationSender::new(),
                            }),
                        );
                    }
                }
            }
        }
    }
}

/// Notification handler for preventing entities from moving through closed entities.
pub fn prevent_moving_through_closed_connections(
    notification: &Notification<VerifyActionNotification, MoveAction>,
    world: &World,
) -> VerifyResult {
    if let Some(location_id) = world
        .get::<Location>(notification.notification_type.performing_entity)
        .map(|location| location.id)
    {
        if let Some(current_location) = world.get::<Container>(location_id) {
            if let Some((connecting_entity, _)) = current_location
                .get_connection_in_direction(&notification.contents.direction, world)
            {
                if let Some(open_state) = world.get::<OpenState>(connecting_entity) {
                    if !open_state.is_open {
                        let message = world
                            .get::<Description>(connecting_entity)
                            .map_or("It's closed.".to_string(), |desc| {
                                format!("The {} is closed.", desc.name)
                            });
                        return VerifyResult::invalid(
                            notification.notification_type.performing_entity,
                            GameMessage::Message(message, MessageDelay::Short),
                        );
                    }
                }
            }
        }
    }

    VerifyResult::valid()
}
