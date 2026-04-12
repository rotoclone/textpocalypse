use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{one_of_literal_part, CommandFormat},
    component::{ActionEndNotification, AfterActionPerformNotification, VerifyResult, Vitals},
    input_parser::{InputParseError, InputParser},
    ActionTag, BeforeActionNotification, GameMessage, StatusEffectsDescription,
    VerifyActionNotification, VitalsDescription, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static VITALS_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(one_of_literal_part(nonempty!["vitals", "vi", "v"])));

pub struct VitalsParser;

impl InputParser for VitalsParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        VITALS_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(VitalsAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![VITALS_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Shows an entity its vitals.
#[derive(Debug)]
struct VitalsAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for VitalsAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let mut result_builder = ActionResult::builder();
        if let Some(vitals) = world.get::<Vitals>(performing_entity) {
            result_builder = result_builder.with_game_message(
                performing_entity,
                GameMessage::Vitals(VitalsDescription::from_vitals(vitals)),
            );
        } else {
            result_builder =
                result_builder.with_error(performing_entity, "You have no vitals.".to_string());
        }

        let status_effects_desc = StatusEffectsDescription::for_entity(performing_entity, world);
        // don't include the status effects message if there are no status effects since it would add extra linebreaks
        if !status_effects_desc.0.is_empty() {
            result_builder = result_builder.with_game_message(
                performing_entity,
                GameMessage::StatusEffects(status_effects_desc),
            );
        }

        result_builder.build_complete_no_tick(true)
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
    ) -> Vec<VerifyResult> {
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
