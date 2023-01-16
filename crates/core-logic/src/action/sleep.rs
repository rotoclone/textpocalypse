use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, Player, SleepState, Vitals},
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// The fraction of energy over which an entity cannot go to sleep if it's awake, and has a chance to wake up if it's asleep.
const WAKE_THRESHOLD: f32 = 0.75;

/// The probability of an entity waking up each tick once it's reached the wake threshold.
const WAKE_CHANCE_PER_TICK: f32 = 0.003;

const SLEEP_FORMAT: &str = "sleep";
const SLEEP_VERB_NAME: &str = "sleep";

lazy_static! {
    static ref SLEEP_PATTERN: Regex = Regex::new("^sleep$").unwrap();
}

pub struct SleepParser;

impl InputParser for SleepParser {
    fn parse(
        &self,
        input: &str,
        entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if SLEEP_PATTERN.is_match(input) {
            if let Some(vitals) = world.get::<Vitals>(entity) {
                let energy_fraction = vitals.energy.get() / vitals.energy.get_max();
                if energy_fraction < WAKE_THRESHOLD {
                    // has vitals, and energy is under wake threshold
                    return Ok(Box::new(SleepAction {
                        ticks_slept: 0,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                } else {
                    // has vitals, but energy not under wake threshold
                    return Err(InputParseError::CommandParseError {
                        verb: SLEEP_VERB_NAME.to_string(),
                        error: CommandParseError::Other(
                            "You're not tired enough to sleep.".to_string(),
                        ),
                    });
                }
            } else {
                // doesn't have vitals
                return Err(InputParseError::CommandParseError {
                    verb: SLEEP_VERB_NAME.to_string(),
                    error: CommandParseError::Other(
                        "You have no energy to regain by sleeping.".to_string(),
                    ),
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SLEEP_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct SleepAction {
    pub ticks_slept: u32,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SleepAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let mut result_builder = ActionResult::builder();

        if self.ticks_slept == 0 {
            fall_asleep(performing_entity, world);
            //TODO add third-person message
            result_builder = result_builder.with_message(
                performing_entity,
                "You close your eyes and drift off to sleep.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Long,
            );
        }

        self.ticks_slept += 1;

        let vitals = world
            .get::<Vitals>(performing_entity)
            .expect("sleeping entity should have vitals");

        let energy_fraction = vitals.energy.get() / vitals.energy.get_max();
        if vitals.energy.get() >= vitals.energy.get_max()
            || (energy_fraction >= WAKE_THRESHOLD
                && rand::thread_rng().gen::<f32>() <= WAKE_CHANCE_PER_TICK)
        {
            wake_up(performing_entity, world);
            //TODO add third-person message
            return result_builder
                .with_message(
                    performing_entity,
                    "You open your eyes.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
                .build_complete_should_tick(true);
        }

        result_builder.build_incomplete(true)
    }

    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult {
        wake_up(performing_entity, world);

        //TODO add third-person message
        ActionInterruptResult::message(
            performing_entity,
            "You wake with a start.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
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

/// Makes an entity be sleeping.
fn fall_asleep(entity: Entity, world: &mut World) {
    if let Some(mut player) = world.get_mut::<Player>(entity) {
        player.message_filter.filter_all_surroundings();
    }

    world
        .entity_mut(entity)
        .insert(SleepState { is_asleep: true });
}

/// Makes an entity be not sleeping.
fn wake_up(entity: Entity, world: &mut World) {
    if let Some(mut player) = world.get_mut::<Player>(entity) {
        player.message_filter.unfilter_all_surroundings();
    }

    world
        .entity_mut(entity)
        .insert(SleepState { is_asleep: false });
}
