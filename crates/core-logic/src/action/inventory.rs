use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, Container},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, ContainerDescription, GameMessage, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const INVENTORY_FORMAT: &str = "inventory";

lazy_static! {
    static ref INVENTORY_PATTERN: Regex = Regex::new("^(i|inv|inventory)$").unwrap();
}

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

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
struct InventoryAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for InventoryAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(container) = world.get::<Container>(performing_entity) {
            let message =
                GameMessage::Container(ContainerDescription::from_container(container, world));

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

    fn send_before_notification(&self, performing_entity: Entity, world: &mut World) {
        self.notification_sender
            .send_before_notification(performing_entity, self, world);
    }

    fn send_verify_notification(
        &self,
        performing_entity: Entity,
        world: &mut World,
    ) -> VerifyResult {
        self.notification_sender
            .send_verify_notification(performing_entity, self, world)
    }

    fn send_after_perform_notification(
        &self,
        performing_entity: Entity,
        action_complete: bool,
        action_successful: bool,
        world: &mut World,
    ) {
        self.notification_sender.send_after_perform_notification(
            performing_entity,
            action_complete,
            action_successful,
            self,
            world,
        );
    }

    fn send_end_notification(
        &self,
        performing_entity: Entity,
        action_interrupted: bool,
        world: &mut World,
    ) {
        self.notification_sender.send_end_notification(
            performing_entity,
            action_interrupted,
            self,
            world,
        );
    }
}
