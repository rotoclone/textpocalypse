use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{
        ActionEndNotification, AfterActionPerformNotification, CombatState, Weapon, WornItems,
    },
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, GameMessage, RangesDescription, VerifyActionNotification, World,
    WornItemsDescription,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const RANGES_FORMAT: &str = "ranges";

lazy_static! {
    static ref RANGES_PATTERN: Regex = Regex::new("^(range|ranges|combat|com)$").unwrap();
}

pub struct RangesParser;

impl InputParser for RangesParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if RANGES_PATTERN.is_match(input) {
            return Ok(Box::new(RangesAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![RANGES_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
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
