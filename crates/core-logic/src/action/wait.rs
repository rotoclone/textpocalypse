use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::AfterActionNotification,
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    time::{HOURS_PER_DAY, MINUTES_PER_HOUR, SECONDS_PER_MINUTE, TICK_DURATION},
    BeforeActionNotification, MessageDelay, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const TICKS_PER_MINUTE: u64 = SECONDS_PER_MINUTE as u64 / TICK_DURATION.as_secs();
const TICKS_PER_HOUR: u64 = TICKS_PER_MINUTE * MINUTES_PER_HOUR as u64;
const TICKS_PER_DAY: u64 = TICKS_PER_HOUR * HOURS_PER_DAY as u64;

const WAIT_VERB_NAME: &str = "wait";
const WAIT_FORMAT: &str = "wait <>";
const WAIT_TIME_CAPTURE: &str = "time";

lazy_static! {
    static ref WAIT_PATTERN: Regex = Regex::new("^wait( (?P<time>.*))?$").unwrap();
    static ref MINUTES_PATTERN: Regex =
        Regex::new("^(\\d+)( )?(m|min|mins|minute|minutes)$").unwrap();
    static ref HOURS_PATTERN: Regex = Regex::new("^(\\d+)( )?(h|hr|hrs|hour|hours)$").unwrap();
    static ref DAYS_PATTERN: Regex = Regex::new("^(\\d+)( )?(d|day|days)$").unwrap();
}

pub struct WaitParser;

impl InputParser for WaitParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = WAIT_PATTERN.captures(input) {
            if let Some(time_match) = captures.name(WAIT_TIME_CAPTURE) {
                let total_ticks_to_wait = parse_time_to_ticks(time_match.as_str())?;
                return Ok(Box::new(WaitAction {
                    total_ticks_to_wait,
                    waited_ticks: 0,
                    notification_sender: ActionNotificationSender::new(),
                }));
            } else {
                return Ok(Box::new(WaitAction {
                    total_ticks_to_wait: 1,
                    waited_ticks: 0,
                    notification_sender: ActionNotificationSender::new(),
                }));
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![WAIT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

enum TimeUnit {
    Minute,
    Hour,
    Day,
}

fn parse_time_to_ticks(time_str: &str) -> Result<u64, InputParseError> {
    let (captures, unit) = if let Some(minutes_caps) = MINUTES_PATTERN.captures(time_str) {
        (minutes_caps, TimeUnit::Minute)
    } else if let Some(hours_caps) = HOURS_PATTERN.captures(time_str) {
        (hours_caps, TimeUnit::Hour)
    } else if let Some(days_caps) = DAYS_PATTERN.captures(time_str) {
        (days_caps, TimeUnit::Day)
    } else {
        return Err(InputParseError::CommandParseError {
            verb: WAIT_VERB_NAME.to_string(),
            error: CommandParseError::Other(
                "You can only wait for some amount of minutes, hours, or days.".to_string(),
            ),
        });
    };

    let amount = if let Some(amount_match) = captures.get(1) {
        match amount_match.as_str().parse::<u64>() {
            Ok(a) => a,
            Err(_) => {
                return Err(InputParseError::CommandParseError {
                    verb: WAIT_VERB_NAME.to_string(),
                    error: CommandParseError::Other(
                        "That is an invalid amount of time to wait.".to_string(),
                    ),
                })
            }
        }
    } else {
        return Err(InputParseError::CommandParseError {
            verb: WAIT_VERB_NAME.to_string(),
            error: CommandParseError::Other("I can't tell how long you want to wait.".to_string()),
        });
    };

    if amount == 0 {
        return Err(InputParseError::CommandParseError {
            verb: WAIT_VERB_NAME.to_string(),
            error: CommandParseError::Other("You can't wait for no amount of time.".to_string()),
        });
    }

    let ticks = match unit {
        TimeUnit::Minute => amount * TICKS_PER_MINUTE,
        TimeUnit::Hour => amount * TICKS_PER_HOUR,
        TimeUnit::Day => amount * TICKS_PER_DAY,
    };

    if ticks > TICKS_PER_DAY {
        return Err(InputParseError::CommandParseError {
            verb: WAIT_VERB_NAME.to_string(),
            error: CommandParseError::Other(
                "You can wait for a maximum of one day at a time.".to_string(),
            ),
        });
    }

    Ok(ticks)
}

#[derive(Debug)]
struct WaitAction {
    total_ticks_to_wait: u64,
    waited_ticks: u64,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for WaitAction {
    fn perform(&mut self, performing_entity: Entity, _: &mut World) -> ActionResult {
        if self.waited_ticks >= self.total_ticks_to_wait {
            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    "You finish waiting.".to_string(),
                    MessageDelay::Short,
                )
                .build_complete_no_tick(true);
        }

        let mut result_builder = ActionResult::builder();

        if self.waited_ticks == 0 {
            result_builder = result_builder.with_message(
                performing_entity,
                "You start waiting...".to_string(),
                MessageDelay::Long,
            );
        }

        self.waited_ticks += 1;
        result_builder.build_incomplete(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop waiting.".to_string(),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
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
