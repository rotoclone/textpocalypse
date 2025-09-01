use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    can_receive_messages,
    command_format::{entity_part, one_of_literal_part, optional_literal_part},
    component::{
        ActionEndNotification, AfterActionPerformNotification, Connection, Container, Description,
        Room,
    },
    game_map::Coordinates,
    input_parser::{input_formats_if_has_component, CommandTarget, InputParseError, InputParser},
    literal_part,
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, CommandFormat, CommandPartId, DetailedEntityDescription,
    EntityDescription, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    RoomDescription, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static LOOK_NO_TARGET_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(one_of_literal_part(nonempty!["look", "l",])));

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static LOOK_WITH_TARGET_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(
        one_of_literal_part(nonempty!["look", "l",]).with_error_string_override("look"),
    )
    .then(literal_part(" ").always_include_in_errors())
    .then(optional_literal_part("at ").always_include_in_errors())
    .then(
        entity_part(TARGET_PART_ID.clone())
            .always_include_in_errors()
            .with_if_missing("what")
            .with_placeholder_for_format_string("thing/direction"),
    )
});
static DETAILED_LOOK_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(
        one_of_literal_part(nonempty!["examine", "ex", "x",]).with_error_string_override("examine"),
    )
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part(TARGET_PART_ID.clone())
            .always_include_in_errors()
            .with_if_missing("what")
            .with_placeholder_for_format_string("thing/direction"),
    )
});

//TODO split into multiple parsers for different formats
pub struct LookParser;

impl InputParser for LookParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if LOOK_NO_TARGET_FORMAT
            .parse(input, source_entity, world)
            .is_ok()
        {
            let here = match CommandTarget::Here.find_target_entity(source_entity, world) {
                Some(e) => e,
                None => {
                    return Err(InputParseError::PostFormatParse(
                        "There's nothing to see.".to_string(),
                    ));
                }
            };

            return Ok(Box::new(LookAction {
                target: here,
                detailed: false,
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        //TODO use `?` instead
        //TODO can't use `?`, because if this fails to parse then `DETAILED_LOOK_COMMAND_FORMAT` could still succeed
        match LOOK_WITH_TARGET_FORMAT.parse(input, source_entity, world) {
            Ok(p) => {
                let target = p.get(&TARGET_PART_ID);

                return Ok(Box::new(LookAction {
                    target,
                    detailed: false,
                    notification_sender: ActionNotificationSender::new(),
                }));
            }
            Err(e) => {
                if e.num_parts_matched() > 0 {
                    return Err(e.into());
                }
            }
        };

        let parsed = DETAILED_LOOK_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(LookAction {
            target: parsed.get(&TARGET_PART_ID),
            detailed: true,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            LOOK_NO_TARGET_FORMAT.get_format_description().to_string(),
            LOOK_WITH_TARGET_FORMAT.get_format_description().to_string(),
            DETAILED_LOOK_FORMAT.get_format_description().to_string(),
        ]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<Description>(
            entity,
            world,
            &[
                LOOK_WITH_TARGET_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world),
                DETAILED_LOOK_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world),
            ],
        )
    }
}

/// Shows an entity the description of something.
#[derive(Debug)]
pub struct LookAction {
    pub target: Entity,
    pub detailed: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for LookAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, performing_entity) {
            return ActionResult::none();
        }

        let target = world.entity(self.target);

        if let Some(room) = target.get::<Room>() {
            if let Some(container) = target.get::<Container>() {
                if let Some(coords) = target.get::<Coordinates>() {
                    return ActionResult::builder()
                        .with_game_message(
                            performing_entity,
                            GameMessage::Room(RoomDescription::from_room(
                                room,
                                container,
                                coords,
                                performing_entity,
                                world,
                            )),
                        )
                        .build_complete_no_tick(true);
                }
            }
        }

        if let Some(desc) = target.get::<Description>() {
            let message = if self.detailed {
                GameMessage::DetailedEntity(DetailedEntityDescription::for_entity(
                    performing_entity,
                    self.target,
                    desc,
                    world,
                ))
            } else {
                GameMessage::Entity(EntityDescription::for_entity(
                    performing_entity,
                    self.target,
                    desc,
                    world,
                ))
            };
            return ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true);
        } else if let Some(connection) = target.get::<Connection>() {
            // the target is a connection without a description, which means it must be just an open connection, so let the performing entity look
            // through it
            let room = world
                .get::<Room>(connection.destination)
                .expect("connection destination should be a room");
            let container = world
                .get::<Container>(connection.destination)
                .expect("connection destination should be a container");
            let coordinates = world
                .get::<Coordinates>(connection.destination)
                .expect("connection destination should have coordinates");
            let room_description_message = GameMessage::Room(RoomDescription::from_room(
                room,
                container,
                coordinates,
                performing_entity,
                world,
            ));
            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    format!("To the {}, you see:", connection.direction),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::None,
                )
                .with_game_message(performing_entity, room_description_message)
                .build_complete_no_tick(true);
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }

    fn interrupt(&self, _: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::none()
    }

    fn may_require_tick(&self) -> bool {
        false
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
