use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, Container, Item, Location},
    find_owning_entity,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    is_living_entity, move_entity,
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification, World,
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

static GET_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(get|take|pick up) (the )?(?P<item>.*)").unwrap());
static GET_FROM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(get|take) (the )?(?P<item>.*) (from|out of) (the )?(?P<container>.*)").unwrap()
});
static PUT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^put (the )?(?P<item>.*) (in|into) (the )?(?P<container>.*)").unwrap()
});
static DROP_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^drop (the )?(?P<item>.*)").unwrap());

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

        /* this is checked in a verify handler, but it needs to also be checked here so you don't get a different error message depending on if the
           other entity actually has the thing you're trying to get
        */
        let source_owned_by_other_living_entity = find_owning_entity(source_container, world)
            .map(|h| h != entity)
            .unwrap_or(false);
        if source_owned_by_other_living_entity
            || (source_container != entity && is_living_entity(source_container, world))
        {
            let source_name =
                Description::get_reference_name(source_container, Some(entity), world);
            let message = format!("You can't get anything from {source_name}.");
            return Err(InputParseError::CommandParseError {
                verb: verb_name,
                error: CommandParseError::Other(message),
            });
        }

        let item = match &item_target {
            CommandTarget::Named(n) => {
                //TODO have better error message if the item exists, but isn't in your inventory or whatever
                match n.find_target_entity_in_container(source_container, entity, world) {
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

        let destination_container = match destination_target.find_target_entity(entity, world) {
            Some(c) => c,
            None => {
                // TODO this error may reveal inventory contents of another entity: you'll get different errors for trying to put something in a container someone else has vs a container they don't have
                return Err(InputParseError::CommandParseError {
                    verb: verb_name,
                    error: CommandParseError::TargetNotFound(destination_target),
                });
            }
        };

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

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        let mut formats =
            input_formats_if_has_component::<Item>(entity, world, &[GET_FORMAT, DROP_FORMAT])
                .unwrap_or_default();
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

/// Makes an entity move an item between containers.
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
        let performing_entity_location = world
            .get::<Location>(performing_entity)
            .expect("performing entity should have a location")
            .id;

        let dynamic_message = if self.destination == performing_entity {
            if self.source == performing_entity_location {
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${entity.Name} ${entity.you:pick/picks} up ${item.name}.")
                        .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("entity".into(), performing_entity)
                        .with_entity("item".into(), self.item),
                )
            } else {
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${entity.Name} ${entity.you:get/gets} ${item.name} from ${source.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("entity".into(), performing_entity)
                        .with_entity("item".into(), self.item)
                        .with_entity("source".into(), self.source),
                )
            }
        } else if self.destination == performing_entity_location {
            DynamicMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                MessageFormat::new("${entity.Name} ${entity.you:drop/drops} ${item.name}.")
                    .expect("message format should be valid"),
                BasicTokens::new()
                    .with_entity("entity".into(), performing_entity)
                    .with_entity("item".into(), self.item),
            )
        } else {
            DynamicMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                MessageFormat::new(
                    "${entity.Name} ${entity.you:put/puts} ${item.name} into ${destination.name}.",
                )
                .expect("message format should be valid"),
                BasicTokens::new()
                    .with_entity("entity".into(), performing_entity)
                    .with_entity("item".into(), self.item)
                    .with_entity("destination".into(), self.destination),
            )
        };

        let result_builder = ActionResult::builder().with_dynamic_message(
            Some(performing_entity),
            DynamicMessageLocation::SourceEntity,
            dynamic_message,
            world,
        );

        // move the entity after third person messages are generated so they refer to the item in the place it was before it moved
        move_entity(self.item, self.destination, world);

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

    fn get_tags(&self) -> HashSet<ActionTag> {
        [].into()
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

    fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_perform_notification(notification_type, self, world);
    }

    fn send_end_notification(&self, notification_type: ActionEndNotification, world: &mut World) {
        self.notification_sender
            .send_end_notification(notification_type, self, world);
    }
}

//TODO automatically equip retrieved items (without taking a tick) if the entity picking them up has enough free hands to equip the item?

/// Verifies that the source and destination entities are containers.
pub fn verify_source_and_destination_are_containers(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let source = notification.contents.source;
    let destination = notification.contents.destination;

    if world.get::<Container>(source).is_none() {
        let source_name = Description::get_reference_name(source, Some(performing_entity), world);
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("{source_name} is not a container.")),
        );
    }

    if world.get::<Container>(destination).is_none() {
        let destination_name =
            Description::get_reference_name(destination, Some(performing_entity), world);
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("{destination_name} is not a container.")),
        );
    }

    VerifyResult::valid()
}

/// Verifies that the item is actually in the source container.
pub fn verify_item_in_source(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let item = notification.contents.item;
    let source = notification.contents.source;

    if let Some(container) = world.get::<Container>(source) {
        if container
            .get_entities(performing_entity, world)
            .contains(&item)
        {
            return VerifyResult::valid();
        }
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);
    let source_name = Description::get_reference_name(source, Some(performing_entity), world);

    let message = if source == performing_entity {
        format!("You don't have {item_name}.")
    } else {
        format!("{item_name} is not in {source_name}.")
    };

    VerifyResult::invalid(performing_entity, GameMessage::Error(message))
}

/// Verifies that the item is not already in the destination container.
pub fn verify_item_not_in_destination(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let item = notification.contents.item;
    let destination = notification.contents.destination;

    if let Some(container) = world.get::<Container>(destination) {
        if !container
            .get_entities(performing_entity, world)
            .contains(&item)
        {
            return VerifyResult::valid();
        }
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);
    let destination_name =
        Description::get_reference_name(destination, Some(performing_entity), world);

    let message = if destination == performing_entity {
        format!("You already have {item_name}.")
    } else {
        format!("{item_name} is already in {destination_name}.")
    };

    VerifyResult::invalid(performing_entity, GameMessage::Error(message))
}

/// Verifies that the source is not owned by a different living entity than the one doing the action.
pub fn verify_source_not_owned_by_other_living_entity(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let source = notification.contents.source;

    let source_owned_by_other_living_entity = find_owning_entity(source, world)
        .map(|h| h != performing_entity)
        .unwrap_or(false);
    if source_owned_by_other_living_entity
        || (source != performing_entity && is_living_entity(source, world))
    {
        let source_name = Description::get_reference_name(source, Some(performing_entity), world);
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("You can't get anything from {source_name}.")),
        );
    }

    VerifyResult::valid()
}

/// Verifies that the destination is not owned by a different living entity than the one doing the action.
pub fn verify_destination_not_owned_by_other_living_entity(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let destination = notification.contents.destination;

    let destination_owned_by_other_living_entity = find_owning_entity(destination, world)
        .map(|h| h != performing_entity)
        .unwrap_or(false);
    if destination_owned_by_other_living_entity
        || (destination != performing_entity && is_living_entity(destination, world))
    {
        let destination_name =
            Description::get_reference_name(destination, Some(performing_entity), world);
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("You can't put anything in {destination_name}.")),
        );
    }

    VerifyResult::valid()
}

/// Prevents putting items inside themselves.
pub fn prevent_put_item_inside_itself(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let item = notification.contents.item;
    let destination = notification.contents.destination;

    if let Some(container) = world.get::<Container>(item) {
        if item == destination
            || container.contains_recursive_including_invisible(destination, world)
        {
            let item_name = Description::get_reference_name(item, Some(performing_entity), world);
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!("You can't put {item_name} inside itself.")),
            );
        }
    }

    VerifyResult::valid()
}

/// Prevents picking up or dropping entities not marked as items.
pub fn prevent_put_non_item(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let item = notification.contents.item;

    if world.get::<Item>(item).is_none() {
        let performing_entity_location = world
            .get::<Location>(performing_entity)
            .expect("performing entity should have a location")
            .id;
        let item_name = Description::get_reference_name(item, Some(performing_entity), world);

        let message = if notification.contents.source == performing_entity_location {
            format!("You can't get {item_name}.")
        } else if notification.contents.destination == performing_entity_location {
            format!("You can't drop {item_name}.")
        } else {
            format!("You can't put {item_name} anywhere.")
        };

        return VerifyResult::invalid(performing_entity, GameMessage::Error(message));
    }

    VerifyResult::valid()
}
