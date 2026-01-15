use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    command_format::{
        any_text_part_with_validator, literal_part, CommandFormat, CommandPartId,
        CommandPartValidateError, CommandPartValidateResult, PartValidatorContext,
    },
    component::{ActionEndNotification, ActionQueue, AfterActionPerformNotification, Player},
    input_parser::{CommandTarget, InputParseError, InputParser},
    notification::{Notification, VerifyResult},
    time::{HOURS_PER_DAY, MINUTES_PER_HOUR, SECONDS_PER_MINUTE, TICK_DURATION},
    ActionTag, BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult, LookAction};

const TICKS_PER_MINUTE: u64 = SECONDS_PER_MINUTE as u64 / TICK_DURATION.as_secs();
const TICKS_PER_HOUR: u64 = TICKS_PER_MINUTE * MINUTES_PER_HOUR as u64;
const TICKS_PER_DAY: u64 = TICKS_PER_HOUR * HOURS_PER_DAY as u64;

static WAIT_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(literal_part("wait")));

static DURATION_PART_ID: LazyLock<CommandPartId<String>> =
    LazyLock::new(|| CommandPartId::new("duration"));
//TODO allow "wait for <duration>"
static WAIT_WITH_DURATION_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("wait"))
        .then(literal_part(" "))
        .then(
            any_text_part_with_validator(DURATION_PART_ID.clone(), validate_duration)
                .with_if_unparsed("how long")
                .with_placeholder_for_format_string("duration"),
        )
});

static MINUTES_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(\\d+)( )?(m|min|mins|minute|minutes)$").unwrap());
static HOURS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(\\d+)( )?(h|hr|hrs|hour|hours)$").unwrap());
static DAYS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(\\d+)( )?(d|day|days)$").unwrap());
//TODO add some way to wait until the only queued actions across all players are wait actions

/// Validates that a string represents an amount of time
fn validate_duration(
    context: &PartValidatorContext<String>,
    _: &World,
) -> CommandPartValidateResult {
    match parse_time_to_ticks(&context.parsed_value) {
        Ok(_) => CommandPartValidateResult::Valid,
        Err(e) => CommandPartValidateResult::Invalid(CommandPartValidateError { details: Some(e) }),
    }
}

pub struct WaitParser;
pub struct WaitWithDurationParser;

impl InputParser for WaitParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        WAIT_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(WaitAction {
            total_ticks_to_wait: 1,
            waited_ticks: 0,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![WAIT_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

impl InputParser for WaitWithDurationParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = WAIT_WITH_DURATION_FORMAT.parse(input, source_entity, world)?;
        let total_ticks_to_wait = parse_time_to_ticks(parsed.get(&DURATION_PART_ID).as_str())
            .map_err(InputParseError::PostFormatParse)?;
        Ok(Box::new(WaitAction {
            total_ticks_to_wait,
            waited_ticks: 0,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![WAIT_WITH_DURATION_FORMAT
            .get_format_description()
            .to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

enum TimeUnit {
    Minute,
    Hour,
    Day,
}

fn parse_time_to_ticks(time_str: &str) -> Result<u64, String> {
    let (captures, unit) = if let Some(minutes_caps) = MINUTES_PATTERN.captures(time_str) {
        (minutes_caps, TimeUnit::Minute)
    } else if let Some(hours_caps) = HOURS_PATTERN.captures(time_str) {
        (hours_caps, TimeUnit::Hour)
    } else if let Some(days_caps) = DAYS_PATTERN.captures(time_str) {
        (days_caps, TimeUnit::Day)
    } else {
        return Err("You can only wait for some amount of minutes, hours, or days.".to_string());
    };

    let amount = if let Some(amount_match) = captures.get(1) {
        match amount_match.as_str().parse::<u64>() {
            Ok(a) => a,
            Err(_) => {
                return Err(format!(
                    "'{}' is an invalid amount of time to wait.",
                    amount_match.as_str()
                ));
            }
        }
    } else {
        return Err("Unable to determine how long you want to wait.".to_string());
    };

    if amount == 0 {
        return Err("You can't wait for no amount of time.".to_string());
    }

    let ticks = match unit {
        TimeUnit::Minute => amount * TICKS_PER_MINUTE,
        TimeUnit::Hour => amount * TICKS_PER_HOUR,
        TimeUnit::Day => amount * TICKS_PER_DAY,
    };

    if ticks > TICKS_PER_DAY {
        return Err("You can wait for a maximum of one day at a time.".to_string());
    }

    Ok(ticks)
}

/// Makes an entity wait for some amount of time.
#[derive(Debug)]
pub struct WaitAction {
    total_ticks_to_wait: u64,
    waited_ticks: u64,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for WaitAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if self.waited_ticks == 0 && self.total_ticks_to_wait == 1 {
            self.waited_ticks = 1;
            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    "You wait for a few seconds.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Long,
                )
                .build_complete_should_tick(true);
        }

        if self.waited_ticks >= self.total_ticks_to_wait {
            remove_wait_filter(performing_entity, world);

            return ActionResult::builder()
                .with_message(
                    performing_entity,
                    "You finish waiting.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
                .build_complete_no_tick(true);
        }

        let mut result_builder = ActionResult::builder();

        if self.waited_ticks == 0 {
            add_wait_filter(performing_entity, world);

            result_builder = result_builder.with_message(
                performing_entity,
                "You start waiting...".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Long,
            );
        }

        self.waited_ticks += 1;
        result_builder.build_incomplete(true)
    }

    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult {
        if self.total_ticks_to_wait > 1 {
            remove_wait_filter(performing_entity, world);
        }

        ActionInterruptResult::message(
            performing_entity,
            "You stop waiting.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
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

/// Applies filters for messages that shouldn't be sent to waiting entities.
fn add_wait_filter(entity: Entity, world: &mut World) {
    if let Some(mut player) = world.get_mut::<Player>(entity) {
        player
            .message_filter
            .filter_all_surroundings_except(&[SurroundingsMessageCategory::Speech]);
    }
}

/// Removes filters for messages that shouldn't be sent to waiting entities.
fn remove_wait_filter(entity: Entity, world: &mut World) {
    if let Some(mut player) = world.get_mut::<Player>(entity) {
        player
            .message_filter
            .unfilter_all_surroundings_except(&[SurroundingsMessageCategory::Speech]);
    }
}

/// Notification handler that queues up a look action after an entity stops waiting, so they can see what's goin on.
pub fn look_on_end_wait(
    notification: &Notification<ActionEndNotification, WaitAction>,
    world: &mut World,
) {
    if notification.contents.total_ticks_to_wait == 1 {
        return;
    }

    let performing_entity = notification.notification_type.performing_entity;
    if let Some(target) = CommandTarget::Here.find_target_entity(performing_entity, world) {
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(LookAction {
                target,
                detailed: false,
                notification_sender: ActionNotificationSender::new(),
            }),
        );
    }
}
