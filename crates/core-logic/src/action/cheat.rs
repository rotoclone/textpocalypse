use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BeforeActionNotification, MessageCategory, MessageDelay, Notification,
    VerifyActionNotification, World, Xp, XpAwardNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const CHEAT_VERB_NAME: &str = "cheat";
const CHEAT_FORMAT: &str = "%<>% <>";
const COMMAND_CAPTURE: &str = "command";
const ARGS_CAPTURE: &str = "args";

static CHEAT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^%(?P<command>.*)%( (?P<args>.*))?").unwrap());

pub struct CheatParser;

impl InputParser for CheatParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = CHEAT_PATTERN.captures(input) {
            if let Some(command_match) = captures.name(COMMAND_CAPTURE) {
                return Ok(Box::new(CheatAction {
                    command: command_match.as_str().to_string(),
                    args: captures
                        .name(ARGS_CAPTURE)
                        .map(|args_match| args_match.as_str().to_string()),
                    notification_sender: ActionNotificationSender::new(),
                }));
            } else {
                return Err(InputParseError::CommandParseError {
                    verb: CHEAT_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![CHEAT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Lets an entity do something they're not allowed to do.
#[derive(Debug)]
pub struct CheatAction {
    pub command: String,
    pub args: Option<String>,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for CheatAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        match self.command.as_str() {
            "give_xp" => give_xp(performing_entity, &self.args, world),
            x => ActionResult::error(performing_entity, format!("Unknown cheat command: {x}")),
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

fn give_xp(entity: Entity, args: &Option<String>, world: &mut World) -> ActionResult {
    if let Some(args) = args {
        match args.parse() {
            Ok(amount) => {
                Notification::send_no_contents(
                    XpAwardNotification {
                        entity,
                        xp_to_add: Xp(amount),
                    },
                    world,
                );

                ActionResult::message(
                    entity,
                    format!("Awarded you {amount} XP."),
                    MessageCategory::System,
                    MessageDelay::None,
                    false,
                )
            }
            Err(e) => ActionResult::error(entity, format!("Error: {e}")),
        }
    } else {
        ActionResult::error(entity, "give_xp requires 1 numeric argument".to_string())
    }
}
