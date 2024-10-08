use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, Container},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, ContainerDescription, GameMessage,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const INVENTORY_FORMAT: &str = "inventory";

static INVENTORY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(i|inv|inventory)$").unwrap());

pub struct InventoryParser;

impl InputParser for InventoryParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if INVENTORY_PATTERN.is_match(input) {
            return Ok(Box::new(InventoryAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![INVENTORY_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Shows an entity its inventory.
#[derive(Debug)]
struct InventoryAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for InventoryAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(container) = world.get::<Container>(performing_entity) {
            let message = GameMessage::Container(ContainerDescription::from_container(
                container,
                performing_entity,
                world,
            ));

            ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true)
        } else {
            ActionResult::error(performing_entity, "You have no inventory.".to_string())
        }
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
