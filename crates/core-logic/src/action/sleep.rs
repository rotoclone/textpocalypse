use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;

use crate::{
    component::{AfterActionNotification, SleepState, Vitals},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    BeforeActionNotification, MessageDelay, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// The fraction of energy over which an entity cannot go to sleep if it's awake, and has a chance to wake up if it's asleep.
const WAKE_THRESHOLD: f32 = 0.8;

/// The probability of an entity waking up each tick once it's reached the wake threshold.
const WAKE_CHANCE_PER_TICK: f32 = 0.01;

const SLEEP_FORMAT: &str = "sleep";

lazy_static! {
    static ref SLEEP_PATTERN: Regex = Regex::new("^sleep$").unwrap();
}

pub struct SleepParser;

impl InputParser for SleepParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if SLEEP_PATTERN.is_match(input) {
            //TODO stop from sleeping if not tired enough or has no vitals
            return Ok(Box::new(SleepAction {
                ticks_slept: 0,
                notification_sender: ActionNotificationSender::new(),
            }));
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
            result_builder = result_builder.with_message(
                performing_entity,
                "You close your eyes and drift off to sleep.".to_string(),
                MessageDelay::Long,
            );
        }

        self.ticks_slept += 1;

        let vitals = world
            .get::<Vitals>(performing_entity)
            .expect("sleeping entity should have vitals");

        let energy_fraction = vitals.energy.get() / vitals.energy.get_max();
        if energy_fraction >= WAKE_THRESHOLD
            && rand::thread_rng().gen::<f32>() <= WAKE_CHANCE_PER_TICK
        {
            wake_up(performing_entity, world);
            return result_builder
                .with_message(
                    performing_entity,
                    "You open your eyes.".to_string(),
                    MessageDelay::Short,
                )
                .build_complete_should_tick(true);
        }

        result_builder.build_incomplete(true)
    }

    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult {
        wake_up(performing_entity, world);

        ActionInterruptResult::message(
            performing_entity,
            "You wake with a start.".to_string(),
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
    world
        .entity_mut(entity)
        .insert(SleepState { is_asleep: true });
}

/// Makes an entity be not sleeping.
fn wake_up(entity: Entity, world: &mut World) {
    world
        .entity_mut(entity)
        .insert(SleepState { is_asleep: false });
}
