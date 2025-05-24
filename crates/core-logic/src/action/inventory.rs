use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{literal_part, one_of_part, CommandFormat, CommandParseError},
    component::{ActionEndNotification, AfterActionPerformNotification, Container},
    input_parser::InputParser,
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, ContainerDescription, GameMessage,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static INVENTORY_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("inventory"),
        literal_part("inv"),
        literal_part("i")
    ]))
});

pub struct InventoryParser;

impl InputParser for InventoryParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        INVENTORY_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(InventoryAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![INVENTORY_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
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
