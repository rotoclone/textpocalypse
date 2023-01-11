use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Container, Location},
    get_reference_name,
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
    move_entity,
    notification::VerifyResult,
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const GET_VERB_NAME: &str = "get";
const PUT_VERB_NAME: &str = "put";
const DROP_VERB_NAME: &str = "drop";

const GET_FORMAT: &str = "get <>";
const GET_FROM_FORMAT: &str = "get <> from <>";
const PUT_FORMAT: &str = "put <> in <>";
const DROP_FORMAT: &str = "drop <>";

const ITEM_CAPTURE: &str = "item";
const CONTAINER_CAPTURE: &str = "container";

lazy_static! {
    static ref GET_PATTERN: Regex = Regex::new("^(get|take|pick up) (the )?(?P<item>.*)").unwrap();
    static ref GET_FROM_PATTERN: Regex =
        Regex::new("^(get|take) (the )?(?P<item>.*) (from|out of) (the )?(?P<container>.*)")
            .unwrap();
    static ref PUT_PATTERN: Regex =
        Regex::new("^put (the )?(?P<item>.*) (in|into) (the )?(?P<container>.*)").unwrap();
    static ref DROP_PATTERN: Regex = Regex::new("^drop (the )?(?P<item>.*)").unwrap();
}

pub struct PutParser;

impl InputParser for PutParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (verb_name, item_target, source_target, destination_target) = parse_targets(input)?;

        let source_container = match source_target.find_target_entity(entity, world) {
            Some(c) => c,
            None => {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::TargetNotFound(source_target),
                });
            }
        };

        if world.get::<Container>(source_container).is_none() {
            let source_container_name = get_reference_name(source_container, Some(entity), world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(format!(
                    "{source_container_name} is not a container."
                )),
            });
        }

        let item = match &item_target {
            CommandTarget::Named(n) => {
                //TODO have better error message if the item exists, but isn't in your inventory or whatever
                match n.find_target_entity_in_container(source_container, world) {
                    Some(e) => e,
                    None => {
                        return Err(InputParseError::CommandParseError {
                            verb: verb_name,
                            error: CommandParseError::TargetNotFound(item_target),
                        });
                    }
                }
            }
            _ => {
                // This will be hit if the target item is not a named item, so if the input was "get me from bag" or something
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::Other("That doesn't make sense.".to_string()),
                });
            }
        };

        let item_name = get_reference_name(item, Some(entity), world);

        if destination_target == CommandTarget::Myself {
            let inventory = world
                .get::<Container>(entity)
                .expect("entity should be a container");
            if inventory.entities.contains(&item) {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::Other(format!("You already have {item_name}.")),
                });
            }
        }

        if source_target == CommandTarget::Myself {
            let inventory = world
                .get::<Container>(entity)
                .expect("entity should be a container");
            if !inventory.entities.contains(&item) {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::Other(format!("You don't have {item_name}.")),
                });
            }
        }

        let destination_container = match destination_target.find_target_entity(entity, world) {
            Some(c) => c,
            None => {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::TargetNotFound(destination_target),
                });
            }
        };

        if world.get::<Container>(destination_container).is_none() {
            let destination_container_name =
                get_reference_name(destination_container, Some(entity), world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(format!(
                    "{destination_container_name} is not a container."
                )),
            });
        }

        if let Some(container) = world.get::<Container>(item) {
            if item == destination_container
                || container.contains_recursive(destination_container, world)
            {
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::Other(format!(
                        "You can't put {item_name} inside itself."
                    )),
                });
            }
        }

        Ok(Box::new(PutAction {
            item,
            source: source_container,
            destination: destination_container,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            GET_FORMAT.to_string(),
            GET_FROM_FORMAT.to_string(),
            PUT_FORMAT.to_string(),
            DROP_FORMAT.to_string(),
        ]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        let mut formats = vec![GET_FORMAT.to_string(), DROP_FORMAT.to_string()];
        if world.get::<Container>(entity).is_some() {
            formats.push(GET_FROM_FORMAT.to_string());
            formats.push(PUT_FORMAT.to_string());
        }

        Some(formats)
    }
}

fn parse_targets(
    input: &str,
) -> Result<(String, CommandTarget, CommandTarget, CommandTarget), InputParseError> {
    //TODO disallow picking up living entities and doors and stuff

    // getting an item from something
    if let Some(captures) = GET_FROM_PATTERN.captures(input) {
        if let Some(item_match) = captures.name(ITEM_CAPTURE) {
            if let Some(container_match) = captures.name(CONTAINER_CAPTURE) {
                let item_target = CommandTarget::parse(item_match.as_str());
                let source_target = CommandTarget::parse(container_match.as_str());
                let destination_target = CommandTarget::Myself;

                return Ok((
                    GET_VERB_NAME.to_string(),
                    item_target,
                    source_target,
                    destination_target,
                ));
            }
        }
    }

    // getting an item from the room
    if let Some(captures) = GET_PATTERN.captures(input) {
        if let Some(item_match) = captures.name(ITEM_CAPTURE) {
            let item_target = CommandTarget::parse(item_match.as_str());
            let source_target = CommandTarget::Here;
            let destination_target = CommandTarget::Myself;

            return Ok((
                GET_VERB_NAME.to_string(),
                item_target,
                source_target,
                destination_target,
            ));
        }
    }

    // putting an item into something
    if let Some(captures) = PUT_PATTERN.captures(input) {
        if let Some(item_match) = captures.name(ITEM_CAPTURE) {
            if let Some(container_match) = captures.name(CONTAINER_CAPTURE) {
                let item_target = CommandTarget::parse(item_match.as_str());
                let source_target = CommandTarget::Myself;
                let destination_target = CommandTarget::parse(container_match.as_str());

                return Ok((
                    PUT_VERB_NAME.to_string(),
                    item_target,
                    source_target,
                    destination_target,
                ));
            }
        }
    }

    // dropping an item
    if let Some(captures) = DROP_PATTERN.captures(input) {
        if let Some(item_match) = captures.name(ITEM_CAPTURE) {
            let item_target = CommandTarget::parse(item_match.as_str());
            let source_target = CommandTarget::Myself;
            let destination_target = CommandTarget::Here;

            return Ok((
                DROP_VERB_NAME.to_string(),
                item_target,
                source_target,
                destination_target,
            ));
        }
    }

    Err(InputParseError::UnknownCommand)
}

#[derive(Debug)]
pub struct PutAction {
    /// The item to move.
    pub item: Entity,
    /// Where the item is.
    pub source: Entity,
    /// Where the item should be.
    pub destination: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for PutAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        // verify that the item is in the expected source location
        let item_location = world
            .get::<Location>(self.item)
            .expect("item should have a location")
            .id;
        if item_location != self.source {
            let item_name = get_reference_name(self.item, Some(performing_entity), world);
            let source_name = get_reference_name(self.source, Some(performing_entity), world);
            return ActionResult::error(
                performing_entity,
                format!("{item_name} is not in {source_name}."),
            );
        }

        let item_name = get_reference_name(self.item, Some(performing_entity), world);

        move_entity(self.item, self.destination, world);

        let performing_entity_location = world
            .get::<Location>(performing_entity)
            .expect("performing entity should have a location")
            .id;

        let mut result_builder = ActionResult::builder();

        //TODO include messages for other entities
        if self.destination == performing_entity {
            if self.source == performing_entity_location {
                result_builder = result_builder.with_message(
                    performing_entity,
                    format!("You pick up {item_name}."),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
            } else {
                let source_name = get_reference_name(self.source, Some(performing_entity), world);
                result_builder = result_builder.with_message(
                    performing_entity,
                    format!("You get {item_name} from {source_name}."),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
            }
        } else if self.destination == performing_entity_location {
            result_builder = result_builder.with_message(
                performing_entity,
                format!("You drop {item_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
        } else {
            let destination_name =
                get_reference_name(self.destination, Some(performing_entity), world);
            result_builder = result_builder.with_message(
                performing_entity,
                format!("You put {item_name} into {destination_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
        }

        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop moving items.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
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
