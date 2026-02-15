use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;

use crate::{
    action::{
        Action, ActionInterruptResult, ActionNotificationSender, ActionResult, MoveAction,
        OpenAction,
    },
    command_format::{
        entity_part_builder, literal_part, validate_parsed_value_has_component, CommandFormat,
        CommandPartId,
    },
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, DynamicMessage, DynamicMessageLocation,
    GameMessage, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    description::DescribeAttributes, ActionEndNotification, ActionQueue,
    AfterActionPerformNotification, AttributeDescriber, AttributeDescription, AttributeDetailLevel,
    Connection, Container, Description, Location, ParseCustomInput,
};

static TARGET_PART_ID: CommandPartId<Entity> = CommandPartId::new("target");
static SLAM_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("slam"))
        .then(literal_part(" "))
        .then(
            entity_part_builder(TARGET_PART_ID)
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<OpenState>(context, "slam", world)
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("door"),
        )
});

struct SlamParser;

impl InputParser for SlamParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = SLAM_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(SlamAction {
            target: parsed.get(TARGET_PART_ID),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SLAM_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<OpenState>(
            entity,
            world,
            &[SLAM_FORMAT.get_format_description().with_targeted_entity(
                TARGET_PART_ID,
                entity,
                world,
            )],
        )
    }
}

#[derive(Debug)]
struct SlamAction {
    target: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for SlamAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);
        let state = match world.get::<OpenState>(self.target) {
            Some(s) => s,
            None => {
                return ActionResult::error(
                    performing_entity,
                    format!("You can't slam {target_name}."),
                );
            }
        };

        if !state.is_open {
            return ActionResult::message(
                performing_entity,
                format!("{target_name} is already closed."),
                MessageCategory::Internal(InternalMessageCategory::Misc),
                MessageDelay::Short,
                false,
            );
        }

        OpenState::set_open(self.target, false, world);

        let mut result_builder = ActionResult::builder()
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new(
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Long,
                    MessageFormat::new("You SLAM ${target.name} closed with a loud bang. You hope you didn't wake up the neighbors.").expect("message format should be valid"),
                    BasicTokens::new().with_entity("target".into(), self.target),
                )
                .only_send_to(performing_entity),
                world,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Long,
                    MessageFormat::new("${slammer.Name} SLAMS ${target.name} closed with a loud bang.").expect("message format should be valid"),
                    BasicTokens::new().with_entity("slammer".into(), performing_entity).with_entity("target".into(), self.target),
                ),
                world,
            );

        if let Some(other_side) = get_other_side(self.target, world) {
            if let Some(other_side_location) = world.get::<Location>(other_side) {
                result_builder = result_builder.with_dynamic_message(
                    Some(performing_entity),
                    DynamicMessageLocation::Location(other_side_location.id),
                    DynamicMessage::new_third_person(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Long,
                        MessageFormat::new("${target.Name} SLAMS closed with a loud bang.")
                            .expect("message format should be valid"),
                        BasicTokens::new().with_entity("target".into(), other_side),
                    ),
                    world,
                )
            }
        }

        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop slamming.".to_string(),
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
        if let Some(other_side_id) = get_other_side(entity, world) {
            if let Some(mut other_side_state) = world.get_mut::<OpenState>(other_side_id) {
                if other_side_state.is_open != should_be_open {
                    other_side_state.is_open = should_be_open;

                    // send messages to entities on the other side
                    // TODO let the actions themselves deal with this? for example, the slam message should be different
                    if let Some(location) = world.get::<Location>(other_side_id) {
                        let open_or_closed = if should_be_open { "open" } else { "closed" };
                        DynamicMessage::new_third_person(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                            MessageDelay::Short,
                            MessageFormat::new("${other_side.Name} swings ${open_or_closed}.")
                                .expect("message format should be valid"),
                            BasicTokens::new()
                                .with_entity("other_side".into(), other_side_id)
                                .with_string("open_or_closed".into(), open_or_closed.to_string()),
                        )
                        .send(
                            None,
                            DynamicMessageLocation::Location(location.id),
                            world,
                        );
                    }
                }
            }
        }
    }
}

impl ParseCustomInput for OpenState {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(SlamParser)]
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
            if let Some((connecting_entity, _)) = location.get_connection_in_direction(
                &notification.contents.direction,
                notification.notification_type.performing_entity,
                world,
            ) {
                if let Some(open_state) = world.get::<OpenState>(connecting_entity) {
                    if !open_state.is_open {
                        ActionQueue::queue_first(
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
            if let Some((connecting_entity, _)) = current_location.get_connection_in_direction(
                &notification.contents.direction,
                notification.notification_type.performing_entity,
                world,
            ) {
                if let Some(open_state) = world.get::<OpenState>(connecting_entity) {
                    if !open_state.is_open {
                        let message = world
                            .get::<Description>(connecting_entity)
                            .map_or("It's closed.".to_string(), |desc| {
                                format!("The {} is closed.", desc.name)
                            });
                        return VerifyResult::invalid(
                            notification.notification_type.performing_entity,
                            GameMessage::Message {
                                content: message,
                                category: MessageCategory::Internal(InternalMessageCategory::Misc),
                                delay: MessageDelay::Short,
                                decorations: Vec::new(),
                            },
                        );
                    }
                }
            }
        }
    }

    VerifyResult::valid()
}

/// Gets the entity representing the other side of this door, if there is one
fn get_other_side(this_side: Entity, world: &World) -> Option<Entity> {
    world
        .get::<Connection>(this_side)
        .and_then(|c| c.other_side)
}
