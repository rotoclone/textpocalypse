use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification, Stats},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, GameMessage, StatsDescription, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const STATS_FORMAT: &str = "stats";

lazy_static! {
    static ref STATS_PATTERN: Regex = Regex::new("^(st|stat|stats)$").unwrap();
}

pub struct StatsParser;

impl InputParser for StatsParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if STATS_PATTERN.is_match(input) {
            return Ok(Box::new(StatsAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![STATS_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Shows an entity its stats.
#[derive(Debug)]
struct StatsAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for StatsAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(stats) = world.get::<Stats>(performing_entity) {
            let message = GameMessage::Stats(StatsDescription::from_stats(stats, world));

            ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true)
        } else {
            ActionResult::error(performing_entity, "You have no stats.".to_string())
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
