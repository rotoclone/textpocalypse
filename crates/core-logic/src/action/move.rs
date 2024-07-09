use std::collections::HashSet;

use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    checks::{CheckModifiers, VsCheckParams, VsParticipant},
    component::{
        ActionEndNotification, ActionQueue, AfterActionPerformNotification, Attribute, CombatState,
        Container, Location, Stats,
    },
    input_parser::{CommandTarget, InputParseError, InputParser},
    move_entity,
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Direction, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ActionResultBuilder,
    LookAction,
};

const MOVE_FORMAT: &str = "go <>";
const MOVE_DIRECTION_CAPTURE: &str = "direction";

lazy_static! {
    static ref MOVE_PATTERN: Regex =
        Regex::new("^((go|move) (to (the )?)?)?(?P<direction>.*)").unwrap();
}

pub struct MoveParser;

impl InputParser for MoveParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = MOVE_PATTERN.captures(input) {
            if let Some(dir_match) = captures.name(MOVE_DIRECTION_CAPTURE) {
                if let Some(direction) = Direction::parse(dir_match.as_str()) {
                    return Ok(Box::new(MoveAction {
                        direction,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![MOVE_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Makes an entity move somewhere.
#[derive(Debug)]
pub struct MoveAction {
    pub direction: Direction,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for MoveAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let current_location_id = world
            .get::<Location>(performing_entity)
            .expect("Moving entity should have a location")
            .id;

        let current_location = world
            .get::<Container>(current_location_id)
            .expect("Moving entity's location should be a container");
        let mut result_builder = ActionResult::builder();
        let mut should_tick = false;
        let mut was_successful = false;

        if let Some((_, connection)) =
            current_location.get_connection_in_direction(&self.direction, performing_entity, world)
        {
            let new_room_id = connection.destination;
            should_tick = true;

            let can_move;
            (result_builder, can_move) = try_escape_combat(
                performing_entity,
                self.direction,
                current_location_id,
                result_builder,
                world,
            );

            if can_move {
                // the moving entity is either not in combat, or has successfully escaped from combat
                move_entity(performing_entity, new_room_id, world);
                was_successful = true;

                result_builder = result_builder
                    .with_message(
                        performing_entity,
                        format!("You walk {}.", self.direction),
                        MessageCategory::Internal(InternalMessageCategory::Action),
                        MessageDelay::Long,
                    )
                    .with_dynamic_message(
                        Some(performing_entity),
                        DynamicMessageLocation::Location(current_location_id),
                        DynamicMessage::new_third_person(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Movement),
                            MessageDelay::Short,
                            MessageFormat::new("${performing_entity.Name} walks ${direction}.")
                                .expect("message format should be valid"),
                            BasicTokens::new()
                                .with_entity("performing_entity".into(), performing_entity)
                                .with_string("direction".into(), self.direction.to_string()),
                        ),
                        world,
                    )
                    .with_dynamic_message(
                        Some(performing_entity),
                        DynamicMessageLocation::Location(new_room_id),
                        DynamicMessage::new_third_person(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Movement),
                            MessageDelay::Short,
                            MessageFormat::new(
                                "${performing_entity.Name} walks in from the ${direction}.",
                            )
                            .expect("message format should be valid"),
                            BasicTokens::new()
                                .with_entity("performing_entity".into(), performing_entity)
                                .with_string(
                                    "direction".into(),
                                    self.direction.opposite().to_string(),
                                ),
                        ),
                        world,
                    );
            }
        } else {
            result_builder = result_builder.with_error(
                performing_entity,
                "You can't move in that direction.".to_string(),
            );
        }

        if should_tick {
            result_builder.build_complete_should_tick(was_successful)
        } else {
            result_builder.build_complete_no_tick(was_successful)
        }
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop moving.".to_string(),
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

/// Makes the provided entity try to escape combat by doing one or more stat checks, and adds messages to the result builder.
/// * If the entity isn't in combat, this will return true without performing a stat check or adding any messages.
/// * If the entity passed all the checks, it will leave combat with everyone and this will return true.
/// * If the entity failed any of the checks, this will return false.
fn try_escape_combat(
    entity: Entity,
    direction: Direction,
    current_location_id: Entity,
    mut result_builder: ActionResultBuilder,
    world: &mut World,
) -> (ActionResultBuilder, bool) {
    let entities_to_escape_from = CombatState::get_entities_in_combat_with(entity, world);

    if entities_to_escape_from.is_empty() {
        return (result_builder, true);
    }

    for entity_to_escape_from in entities_to_escape_from.keys() {
        let (check_result, _) = Stats::check_vs(
            VsParticipant {
                entity,
                stat: Attribute::Agility.into(),
                modifiers: CheckModifiers::none(),
            },
            VsParticipant {
                entity: *entity_to_escape_from,
                stat: Attribute::Agility.into(),
                modifiers: CheckModifiers::none(),
            },
            VsCheckParams::second_wins_ties(),
            world,
        );

        if !check_result.succeeded() {
            result_builder = result_builder
                .with_message(
                    entity,
                    format!("You try to escape to the {direction}, but can't get away!",),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
                .with_dynamic_message(
                    Some(entity),
                    DynamicMessageLocation::Location(current_location_id),
                    DynamicMessage::new_third_person(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Movement),
                        MessageDelay::Short,
                        MessageFormat::new("${entity.Name} tries to escape to the ${direction}, but can't get away.")
                                .expect("message format should be valid"),
                            BasicTokens::new()
                                .with_entity("entity".into(), entity)
                                .with_string("direction".into(), direction.to_string()),
                    ),
                    world,
                );
            return (result_builder, false);
        }
    }

    CombatState::leave_all_combat(entity, world);
    result_builder = result_builder.with_message(
        entity,
        format!("You manage to escape to the {direction}!",),
        MessageCategory::Internal(InternalMessageCategory::Action),
        MessageDelay::Short,
    );
    (result_builder, true)
}

/// Notification handler that queues up a look action after an entity moves, so they can see where they ended up.
pub fn look_after_move(
    notification: &Notification<AfterActionPerformNotification, MoveAction>,
    world: &mut World,
) {
    if !notification.notification_type.action_successful
        || !notification.notification_type.action_complete
    {
        return;
    }

    let performing_entity = notification.notification_type.performing_entity;
    if let Some(target) = CommandTarget::Here.find_target_entity(performing_entity, world) {
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(LookAction {
                target,
                detailed: false,
                notification_sender: ActionNotificationSender::new(),
            }),
        );
    }
}
