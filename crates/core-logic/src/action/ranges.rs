use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{one_of_literal_part, CommandFormat},
    component::{
        ActionEndNotification, AfterActionPerformNotification, CombatState, VerifyResult, Weapon,
    },
    input_parser::{InputParseError, InputParser},
    ActionTag, BeforeActionNotification, GameMessage, RangesDescription, VerifyActionNotification,
    World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static RANGES_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "ranges", "range", "combat", "com",
    ]))
});

pub struct RangesParser;

impl InputParser for RangesParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        RANGES_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(RangesAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![RANGES_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Shows an entity the ranges to entities it's in combat with.
#[derive(Debug)]
pub struct RangesAction {
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for RangesAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let combatants = CombatState::get_entities_in_combat_with(performing_entity, world);
        if combatants.is_empty() {
            ActionResult::error(
                performing_entity,
                "You're not in combat with anyone.".to_string(),
            )
        } else {
            let weapon_ranges =
                Weapon::get_primary(performing_entity, world).map(|(weapon, _)| &weapon.ranges);
            let message = GameMessage::Ranges(RangesDescription::from_combatants(
                combatants,
                weapon_ranges,
                world,
            ));

            ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true)
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
