use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Container, Location},
    get_reference_name,
    input_parser::{CommandParseError, CommandTarget, InputParseError, InputParser},
    move_entity,
    notification::VerifyResult,
    BeforeActionNotification, MessageDelay, VerifyActionNotification, World,
};

use super::{Action, ActionNotificationSender, ActionResult};

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
    static ref GET_PATTERN: Regex =
        Regex::new("^get (the )?(?P<item>.*)( from (the )?(?P<container>.*))?").unwrap();
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
        // getting an item
        if let Some(captures) = GET_PATTERN.captures(input) {
            if let Some(item_match) = captures.name(ITEM_CAPTURE) {
                let item_target = CommandTarget::parse(item_match.as_str());
                let item = match &item_target {
                    CommandTarget::Named(n) => match n.find_target_entity(entity, world) {
                        Some(e) => e,
                        None => {
                            return Err(InputParseError::CommandParseError {
                                verb: GET_VERB_NAME.to_string(),
                                error: CommandParseError::TargetNotFound(item_target),
                            });
                        }
                    },
                    _ => {
                        return Err(InputParseError::CommandParseError {
                            verb: GET_VERB_NAME.to_string(),
                            error: CommandParseError::Other("You can't get that.".to_string()),
                        });
                    }
                };

                //TODO handle getting items from containers

                let item_name = get_reference_name(item, world);
                let inventory = world
                    .get::<Container>(entity)
                    .expect("entity should be a container");
                if inventory.entities.contains(&item) {
                    return Err(InputParseError::CommandParseError {
                        verb: GET_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!("You already have {item_name}.")),
                    });
                }

                return Ok(Box::new(PutAction {
                    item,
                    destination: entity,
                    notification_sender: ActionNotificationSender::new(),
                }));
            }
        }

        // TODO handle putting items into containers

        // dropping an item
        if let Some(captures) = DROP_PATTERN.captures(input) {
            if let Some(item_match) = captures.name(ITEM_CAPTURE) {
                let item_target = CommandTarget::parse(item_match.as_str());
                let item = match &item_target {
                    CommandTarget::Named(n) => match n.find_target_entity(entity, world) {
                        Some(e) => e,
                        None => {
                            return Err(InputParseError::CommandParseError {
                                verb: GET_VERB_NAME.to_string(),
                                error: CommandParseError::TargetNotFound(item_target),
                            });
                        }
                    },
                    _ => {
                        return Err(InputParseError::CommandParseError {
                            verb: GET_VERB_NAME.to_string(),
                            error: CommandParseError::Other("You can't drop that.".to_string()),
                        });
                    }
                };

                let item_name = get_reference_name(item, world);
                let inventory = world
                    .get::<Container>(entity)
                    .expect("entity should be a container");
                if !inventory.entities.contains(&item) {
                    return Err(InputParseError::CommandParseError {
                        verb: GET_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!("You don't have {item_name}.")),
                    });
                }

                let destination = world
                    .get::<Location>(entity)
                    .expect("entity should have a location")
                    .id;

                return Ok(Box::new(PutAction {
                    item,
                    destination,
                    notification_sender: ActionNotificationSender::new(),
                }));
            }
        }

        Err(InputParseError::UnknownCommand)
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

#[derive(Debug)]
struct PutAction {
    item: Entity,
    destination: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for PutAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        move_entity(self.item, self.destination, world);

        let item_name = get_reference_name(self.item, world);
        let performing_entity_location = world
            .get::<Location>(performing_entity)
            .expect("performing entity should have a location")
            .id;

        let mut result_builder = ActionResult::builder();

        if self.destination == performing_entity {
            result_builder = result_builder.with_message(
                performing_entity,
                format!("You pick up {item_name}."),
                MessageDelay::Short,
            )
        } else if self.destination == performing_entity_location {
            result_builder = result_builder.with_message(
                performing_entity,
                format!("You drop {item_name}."),
                MessageDelay::Short,
            )
        }

        result_builder.build_complete_should_tick(true)
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
