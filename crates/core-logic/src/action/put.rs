use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{
        entity_part_with_validator, literal_part, one_of_part, validate_parsed_value_has_component,
        CommandFormat, CommandParseError, CommandPartId,
    },
    component::{ActionEndNotification, AfterActionPerformNotification, Container, Item, Location},
    find_owning_entity,
    input_parser::InputParser,
    is_living_entity, move_entity,
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static ITEM_PART_ID: LazyLock<CommandPartId<Entity>> = LazyLock::new(|| CommandPartId::new("item"));
static CONTAINER_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("container"));

static GET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("get"),
        literal_part("take"),
        literal_part("pick up")
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part_with_validator(ITEM_PART_ID.clone(), |context, world| {
            validate_parsed_value_has_component::<Item>(context, "get", world)
        })
        .always_include_in_errors()
        .with_if_missing("what")
        .with_placeholder_for_format_string("item"),
    )
});

static DROP_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("drop"))
        .then(literal_part(" ").always_include_in_errors())
        .then(
            entity_part_with_validator(ITEM_PART_ID.clone(), |context, world| {
                validate_parsed_value_has_component::<Item>(context, "drop", world)
            })
            .always_include_in_errors()
            .with_if_missing("what")
            .with_placeholder_for_format_string("item"),
        )
});

//TODO this doesn't work because the target part doesn't know to look in the container for the target, since parsing happens in a single pass so that part won't have been parsed yet
static GET_FROM_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("get"),
        literal_part("take")
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part_with_validator(ITEM_PART_ID.clone(), |context, world| {
            validate_parsed_value_has_component::<Item>(context, "get", world)
        })
        .always_include_in_errors()
        .with_if_missing("what")
        .with_placeholder_for_format_string("item"),
    )
    .then(literal_part(" ").always_include_in_errors())
    .then(
        one_of_part(nonempty![literal_part("from"), literal_part("out of")])
            .always_include_in_errors(),
    )
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part_with_validator(CONTAINER_PART_ID.clone(), |context, world| {
            validate_parsed_value_has_component::<Container>(context, "get anything from", world)
        })
        .always_include_in_errors()
        .with_if_missing("where")
        .with_placeholder_for_format_string("container"),
    )
});

static PUT_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("put"))
        .then(literal_part(" "))
        .then(
            entity_part_with_validator(ITEM_PART_ID.clone(), |context, world| {
                //TODO ideally the error here would be "you can't put <item name> anywhere" instead of just "you can't put <item name>"
                validate_parsed_value_has_component::<Item>(context, "put", world)
            })
            .with_if_missing("what")
            .with_placeholder_for_format_string("item"),
        )
        .then(literal_part(" "))
        .then(one_of_part(nonempty![
            literal_part("into"),
            literal_part("in")
        ]))
        .then(literal_part(" "))
        .then(
            entity_part_with_validator(CONTAINER_PART_ID.clone(), |context, world| {
                validate_parsed_value_has_component::<Container>(
                    context,
                    "put anything into",
                    world,
                )
            })
            .with_if_missing("where")
            .with_placeholder_for_format_string("container"),
        )
});

pub struct GetParser;
pub struct DropParser;
pub struct GetFromParser;
pub struct PutParser;

impl InputParser for GetParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = GET_FORMAT.parse(input, entity, world)?;

        let item = parsed.get(&ITEM_PART_ID);
        let Some(source_location) = world.get::<Location>(entity) else {
            return Err(CommandParseError::Other("You aren't anywhere.".to_string()));
        };
        let source = source_location.id;
        let destination = entity;

        if item == entity {
            return Err(CommandParseError::Other(
                "You can't get yourself. At least not in a physical sense.".to_string(),
            ));
        }

        Ok(Box::new(PutAction {
            item,
            source,
            destination,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![GET_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String> {
        if world.get::<Item>(entity).is_some()
            && find_owning_entity(entity, world) != Some(pov_entity)
        {
            vec![GET_FORMAT
                .get_format_description()
                .with_targeted_entity(ITEM_PART_ID.clone(), entity, world)
                .to_string()]
        } else {
            Vec::new()
        }
    }
}

impl InputParser for DropParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = DROP_FORMAT.parse(input, entity, world)?;

        let item = parsed.get(&ITEM_PART_ID);
        let source = entity;
        let Some(destination_location) = world.get::<Location>(entity) else {
            return Err(CommandParseError::Other("You aren't anywhere.".to_string()));
        };
        let destination = destination_location.id;

        if item == entity {
            return Err(CommandParseError::Other(
                "You can't drop yourself.".to_string(),
            ));
        }

        Ok(Box::new(PutAction {
            item,
            source,
            destination,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![DROP_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String> {
        if world.get::<Item>(entity).is_some()
            && find_owning_entity(entity, world) == Some(pov_entity)
        {
            vec![DROP_FORMAT
                .get_format_description()
                .with_targeted_entity(ITEM_PART_ID.clone(), entity, world)
                .to_string()]
        } else {
            Vec::new()
        }
    }
}

impl InputParser for GetFromParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = GET_FROM_FORMAT.parse(input, entity, world)?;

        let item = parsed.get(&ITEM_PART_ID);
        let source = parsed.get(&CONTAINER_PART_ID);
        let destination = entity;

        if item == entity {
            return Err(CommandParseError::Other(
                "You can't get yourself. At least not in a physical sense.".to_string(),
            ));
        }

        if item == source {
            let item_name = Description::get_reference_name(item, Some(entity), world);
            return Err(CommandParseError::Other(format!(
                "You can't take {item_name} out of itself."
            )));
        }

        /* this is checked in a verify handler, but it needs to also be checked here so you don't get a different error message depending on if the
           other entity actually has the thing you're trying to get
        */
        let source_owned_by_other_living_entity = find_owning_entity(source, world)
            .map(|h| h != entity)
            .unwrap_or(false);
        if source_owned_by_other_living_entity
            || (source != entity && is_living_entity(source, world))
        {
            let source_name = Description::get_reference_name(source, Some(entity), world);
            return Err(CommandParseError::Other(format!(
                "You can't get anything from {source_name}."
            )));
        }

        Ok(Box::new(PutAction {
            item,
            source,
            destination,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![GET_FROM_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String> {
        let mut formats = Vec::new();

        if world.get::<Item>(entity).is_some()
            && find_owning_entity(entity, world) != Some(pov_entity)
        {
            formats.push(
                GET_FROM_FORMAT
                    .get_format_description()
                    .with_targeted_entity(ITEM_PART_ID.clone(), entity, world)
                    .to_string(),
            )
        }

        if world.get::<Container>(entity).is_some() {
            formats.push(
                GET_FROM_FORMAT
                    .get_format_description()
                    .with_targeted_entity(CONTAINER_PART_ID.clone(), entity, world)
                    .to_string(),
            );
        }

        formats
    }
}

impl InputParser for PutParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = PUT_FORMAT.parse(input, entity, world)?;

        let item = parsed.get(&ITEM_PART_ID);
        let source = entity;
        let destination = parsed.get(&CONTAINER_PART_ID);

        if item == entity {
            return Err(CommandParseError::Other(
                "You can't put yourself anywhere.".to_string(),
            ));
        }

        if item == destination {
            let item_name = Description::get_reference_name(item, Some(entity), world);
            return Err(CommandParseError::Other(format!(
                "You can't put {item_name} inside itself."
            )));
        }

        //TODO ensure the destination isn't a living entity or a container a living entity owns, similar to the check in GetFromParser

        Ok(Box::new(PutAction {
            item,
            source,
            destination,
            notification_sender: ActionNotificationSender::new(),
        }))

        /* TODO
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
        */
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![PUT_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String> {
        let mut formats = Vec::new();

        if world.get::<Item>(entity).is_some()
            && find_owning_entity(entity, world) == Some(pov_entity)
        {
            formats.push(
                PUT_FORMAT
                    .get_format_description()
                    .with_targeted_entity(ITEM_PART_ID.clone(), entity, world)
                    .to_string(),
            )
        }

        if world.get::<Container>(entity).is_some() {
            formats.push(
                PUT_FORMAT
                    .get_format_description()
                    .with_targeted_entity(CONTAINER_PART_ID.clone(), entity, world)
                    .to_string(),
            );
        }

        formats
    }
}

/* TODO remove
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
    */

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
/// TODO remove since the validators cover this now
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
/// TODO remove since the validators cover this now
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flume::{Receiver, Sender};

    use crate::{
        component::Room, game_map::Coordinates, test_utils::spawn_entity_in_location,
        world_setup::spawn_room, Color, Game, GameOptions, MapIcon, SpawnRoom, Time,
    };

    use super::*;

    struct TestGame {
        game: Game,
        item_entity: Entity,
        item_entity_in_container: Entity,
        container_entity: Entity,
        room: Entity,
        player_1: TestPlayer,
        player_2: TestPlayer,
    }

    //TODO move this to a common place probably
    struct TestPlayer {
        entity: Entity,
        command_sender: Sender<String>,
        message_receiver: Receiver<(GameMessage, Time)>,
    }

    #[test]
    fn get_no_target() {
        let mut game = set_up_game();
        test_error("get", "get what?", &mut game);
    }

    #[test]
    fn get_no_target_with_space() {
        let mut game = set_up_game();
        test_error("get ", "get what?", &mut game);
    }

    #[test]
    fn get_target_does_not_exist() {
        let mut game = set_up_game();
        test_error(
            "get blorp",
            "get what? (There's no 'blorp' here.)",
            &mut game,
        );
    }

    #[test]
    fn get_target_self() {
        let mut game = set_up_game();
        test_error("get me", "get what? (You can't get you.)", &mut game);
    }

    #[test]
    fn get_target_location() {
        let mut game = set_up_game();
        //TODO make the error include the name of the room
        test_error("get here", "get what? (You can't get it.)", &mut game);
    }

    #[test]
    fn get_target_not_item() {
        let mut game = set_up_game();
        test_error(
            "get entity non_item name",
            "get what? (You can't get the entity non_item name.)",
            &mut game,
        );
    }

    #[test]
    fn get_target_not_item_with_alias() {
        let mut game = set_up_game();
        test_error(
            "get entity non_item alias 1",
            "get what? (You can't get the entity non_item name.)",
            &mut game,
        )
    }

    #[test]
    fn get_with_container_target_does_not_exist() {
        let mut game = set_up_game();
        //TODO this fails because parsing ends as soon as 'blorp' isn't found, which maybe is fine
        test_error(
            "get blorp from entity container name",
            "get what? (There's no 'blorp' in the entity container name.)",
            &mut game,
        )
    }

    #[test]
    fn get_target_in_container() {
        let mut game = set_up_game();
        test_error(
            "get entity item_in_container name",
            "get what? (There's no 'entity item_in_container name' here.)",
            &mut game,
        )
    }

    #[test]
    fn get_from_but_no_container_name() {
        let mut game = set_up_game();
        test_error(
            "get entity item_in_container name from",
            "get 'entity item_in_container name' from where?",
            &mut game,
        )
    }

    #[test]
    fn get_container_does_not_exist() {
        let mut game = set_up_game();
        test_error(
            "get entity item_in_container name from blorp",
            "get 'entity item_in_container name' from where? (There's no 'blorp' here.)",
            &mut game,
        )
    }

    #[test]
    fn get_with_container_target_not_in_container() {
        let mut game = set_up_game();
        test_error(
            "get entity item name from entity container name",
            "get the entity item name from where? (The entity item name isn't in the entity container name.)",
            &mut game
        )
    }

    #[test]
    fn get_already_have_non_item_target() {
        let mut game = set_up_game();
        let mut world = game.game.world.write().unwrap();
        spawn_entity_in_location("owned", game.player_1.entity, &mut world);
        drop(world);

        test_error(
            "get entity owned name",
            "You can't get your entity owned name.",
            &mut game,
        );
    }

    #[test]
    fn get_already_have_target() {
        let mut game = set_up_game();
        let mut world = game.game.world.write().unwrap();
        let owned_entity = spawn_entity_in_location("owned", game.player_1.entity, &mut world);
        world
            .entity_mut(owned_entity)
            .insert(Item::new_one_handed());
        drop(world);

        test_error(
            "get entity owned name",
            "You already have the entity owned name.",
            &mut game,
        );
    }

    #[test]
    fn get_valid_target() {
        let mut game = set_up_game();
        test_success(
            "get entity item name",
            "You pick up the entity item name.",
            "Player 1 picks up the entity item name.",
            &mut game,
        );

        let world = game.game.world.read().unwrap();

        let location = world.get::<Location>(game.item_entity).unwrap();
        assert_eq!(game.player_1.entity, location.id);

        let player_container = world.get::<Container>(game.player_1.entity).unwrap();
        assert!(player_container
            .get_entities_including_invisible()
            .contains(&game.item_entity));

        let room_container = world.get::<Container>(game.room).unwrap();
        assert!(!room_container
            .get_entities_including_invisible()
            .contains(&game.item_entity));
    }

    #[test]
    fn get_valid_target_from_container() {
        let mut game = set_up_game();
        test_success(
            "get entity item_in_container name from entity container name",
            "You get the entity item_in_container name from the entity container name.",
            "Player 1 gets their entity item_in_container name from their entity container name.",
            &mut game,
        );

        let world = game.game.world.read().unwrap();
        let location = world
            .get::<Location>(game.item_entity_in_container)
            .unwrap();
        assert_eq!(game.player_1.entity, location.id);

        let player_container = world.get::<Container>(game.player_1.entity).unwrap();
        assert!(player_container
            .get_entities_including_invisible()
            .contains(&game.item_entity_in_container));

        let container_container = world.get::<Container>(game.container_entity).unwrap();
        assert!(!container_container
            .get_entities_including_invisible()
            .contains(&game.item_entity_in_container));
    }

    //TODO tests for drop

    //TODO tests for put

    /// Asserts that the provided input results in the provided error
    fn test_error(input: &str, expected_error: &str, game: &mut TestGame) {
        let message_receiver = &game.player_1.message_receiver;
        let command_sender = &game.player_1.command_sender;

        // skip past any intro messages (like a description of the the player spawned in)
        message_receiver.drain();
        command_sender.send(input.to_string()).unwrap();
        let message = message_receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap();

        let GameMessage::Error(actual_error) = message.0 else {
            panic!("Message was not an error: {:?}", message.0);
        };
        assert_eq!(expected_error, actual_error);
    }

    /// Asserts that the provided input results in the provided message
    fn test_success(
        input: &str,
        expected_message: &str,
        expected_third_person_message: &str,
        game: &mut TestGame,
    ) {
        let p1_message_receiver = &game.player_1.message_receiver;
        let p1_command_sender = &game.player_1.command_sender;

        let p2_message_receiver = &game.player_2.message_receiver;
        let p2_command_sender = &game.player_2.command_sender;

        // skip past any intro messages (like a description of the the player spawned in)
        p1_message_receiver.drain();
        p2_message_receiver.drain();

        p1_command_sender.send(input.to_string()).unwrap();
        assert_message_received(p1_message_receiver, "Action queued.");

        p2_command_sender.send("wait".to_string()).unwrap();

        assert_message_received(p1_message_receiver, expected_message);
        assert_message_received(p2_message_receiver, expected_third_person_message);
    }

    fn assert_message_received(
        message_receiver: &Receiver<(GameMessage, Time)>,
        expected_message: &str,
    ) {
        let message = message_receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap();

        let GameMessage::Message {
            content: actual_message,
            ..
        } = message.0
        else {
            panic!("Message was not a message: {:?}", message.0);
        };
        assert_eq!(expected_message, actual_message);
    }

    fn set_up_game() -> TestGame {
        let mut game = Game::new(GameOptions {
            skip_worldgen: true,
            ..GameOptions::default()
        });
        let mut world = game.world.write().unwrap();
        let room_coords = Coordinates {
            x: 0,
            y: 0,
            z: 0,
            parent: None,
        };
        let room = spawn_room(
            Room {
                name: "room".to_string(),
                description: "it's a room".to_string(),
                map_icon: MapIcon::new_uniform(Color::Black, Color::White, ['[', ']']),
            },
            room_coords.clone(),
            &mut world,
        );
        world.insert_resource(SpawnRoom(room_coords));
        spawn_entity_in_location("non_item", room, &mut world);

        let item_entity = spawn_entity_in_location("item", room, &mut world);
        world.entity_mut(item_entity).insert(Item::new_one_handed());

        let container_entity = spawn_entity_in_location("container", room, &mut world);
        world
            .entity_mut(container_entity)
            .insert(Container::new_infinite());

        let item_entity_in_container =
            spawn_entity_in_location("item_in_container", container_entity, &mut world);
        world
            .entity_mut(item_entity_in_container)
            .insert(Item::new_one_handed());
        drop(world);

        let (p1_entity, p1_command_sender, p1_message_receiver) =
            game.add_player("player 1".to_string());
        let (p2_entity, p2_command_sender, p2_message_receiver) =
            game.add_player("player 2".to_string());

        TestGame {
            game,
            item_entity,
            item_entity_in_container,
            container_entity,
            room,
            player_1: TestPlayer {
                entity: p1_entity,
                command_sender: p1_command_sender,
                message_receiver: p1_message_receiver,
            },
            player_2: TestPlayer {
                entity: p2_entity,
                command_sender: p2_command_sender,
                message_receiver: p2_message_receiver,
            },
        }
    }
}
