use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, GameMessage, PlayersMessage, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const PLAYERS_FORMAT: &str = "players";

lazy_static! {
    static ref PLAYERS_PATTERN: Regex = Regex::new("^(pl|players)$").unwrap();
}

pub struct PlayersParser;

impl InputParser for PlayersParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if PLAYERS_PATTERN.is_match(input) {
            return Ok(Box::new(PlayersAction {
                notification_sender: ActionNotificationSender::new(),
            }));
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![PLAYERS_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct PlayersAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for PlayersAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let message = GameMessage::Players(PlayersMessage::for_entity(performing_entity, world));

        ActionResult::builder()
            .with_game_message(performing_entity, message)
            .build_complete_no_tick(true)
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
