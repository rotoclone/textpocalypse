use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Vitals},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, GameMessage, VerifyActionNotification, VitalsDescription, World,
};

use super::{Action, ActionNotificationSender, ActionResult};

const VITALS_FORMAT: &str = "vitals";

lazy_static! {
    static ref VITALS_PATTERN: Regex = Regex::new("^(v|vi|vitals)$").unwrap();
}

pub struct VitalsParser;

impl InputParser for VitalsParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if VITALS_PATTERN.is_match(input) {
            return Ok(Box::new(VitalsAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![VITALS_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
struct VitalsAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for VitalsAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(vitals) = world.get::<Vitals>(performing_entity) {
            let message = GameMessage::Vitals(VitalsDescription::from_vitals(vitals));

            ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true)
        } else {
            ActionResult::error(performing_entity, "You have no vitals.".to_string())
        }
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

    fn send_after_notification(
        &self,
        notification_type: AfterActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_notification(notification_type, self, world);
    }
}
