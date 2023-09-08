use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{ThirdPersonMessage, ThirdPersonMessageLocation},
    checks::{CheckDifficulty, CheckResult},
    component::{
        ActionEndNotification, AfterActionPerformNotification, Attribute, Item, Location, Stats,
        Weight,
    },
    get_article_reference_name, get_reference_name, get_volume, get_weight,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    is_living_entity, move_entity,
    notification::VerifyResult,
    value_change::{ValueChange, ValueChangeOperation},
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, ValueType, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// The number of kilograms an entity can throw per point of strength they have.
const KG_CAN_THROW_PER_STRENGTH: f32 = 2.0;

/// The amount of damage to do to an entity that is hit by a thrown object per kilogram the thrown object weighs.
const HIT_DAMAGE_PER_KG: f32 = 3.0;

/// The amount to multiply the hit damage by if it's a really good throw.
const DIRECT_HIT_DAMAGE_MULT: f32 = 2.0;

/// The amount to multiply the difficulty multiplier of a throw check by for hitting an inanimate object with a volume of 1 liter
const VOLUME_DIFFICULTY_MULT_MULT: f32 = 2.0;

/// The base difficutly of a throw check per kilogram the thrown object weighs
const DIFFICULTY_PER_KG: f32 = 1.0;

/// The minimum base difficulty of throw checks
const MIN_BASE_DIFFICULTY: f32 = 5.0;

/// The minimum amount to multiply throw check difficulty by due to the size of the target
const MIN_VOLUME_DIFFICULTY_MULT: f32 = 0.33;

/// The maximum amount to multiply throw check difficulty by due to the size of the target
const MAX_VOLUME_DIFFICULTY_MULT: f32 = 3.0;

const THROW_VERB_NAME: &str = "throw";
const THROW_FORMAT: &str = "throw <> at <>";
const NAME_CAPTURE: &str = "name";
const TARGET_CAPTURE: &str = "target";

lazy_static! {
    static ref THROW_PATTERN: Regex =
        Regex::new("^throw (the )?(?P<name>.*) at (the )?(?P<target>.*)").unwrap();
}

pub struct ThrowParser;

impl InputParser for ThrowParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = THROW_PATTERN.captures(input) {
            if let Some(item_match) = captures.name(NAME_CAPTURE) {
                // item to throw was provided
                if let Some(target_match) = captures.name(TARGET_CAPTURE) {
                    // target was provided
                    let item = CommandTarget::parse(item_match.as_str());
                    if let Some(item_entity) = item.find_target_entity(source_entity, world) {
                        // item to throw exists
                        let target = CommandTarget::parse(target_match.as_str());
                        if let Some(target_entity) = target.find_target_entity(source_entity, world)
                        {
                            // target exists
                            if target_entity == item_entity {
                                let item_name =
                                    get_reference_name(item_entity, Some(source_entity), world);
                                return Err(InputParseError::CommandParseError {
                                    verb: THROW_VERB_NAME.to_string(),
                                    error: CommandParseError::Other(format!(
                                        "You can't throw {item_name} at itself."
                                    )),
                                });
                            }

                            if target_entity == source_entity {
                                return Err(InputParseError::CommandParseError {
                                    verb: THROW_VERB_NAME.to_string(),
                                    error: CommandParseError::Other(
                                        "You can't throw things at yourself.".to_string(),
                                    ),
                                });
                            }

                            match get_cannot_throw_reason(source_entity, item_entity, world) {
                                Some(CannotThrowReason::NotThrowable) => {
                                    let item_name =
                                        get_reference_name(item_entity, Some(source_entity), world);
                                    return Err(InputParseError::CommandParseError {
                                        verb: THROW_VERB_NAME.to_string(),
                                        error: CommandParseError::Other(format!(
                                            "You can't throw {item_name}."
                                        )),
                                    });
                                }
                                Some(CannotThrowReason::TooWeak) => {
                                    let item_name =
                                        get_reference_name(item_entity, Some(source_entity), world);
                                    return Err(InputParseError::CommandParseError {
                                        verb: THROW_VERB_NAME.to_string(),
                                        error: CommandParseError::Other(format!(
                                            "You aren't strong enough to throw {item_name}."
                                        )),
                                    });
                                }
                                None => {
                                    // item to throw is throwable
                                    if world.get::<Item>(target_entity).is_some()
                                        || is_living_entity(target_entity, world)
                                    {
                                        // target is valid
                                        return Ok(Box::new(ThrowAction {
                                            item: item_entity,
                                            target: target_entity,
                                            notification_sender: ActionNotificationSender::new(),
                                        }));
                                    } else {
                                        // target is not valid
                                        let target_name = get_reference_name(
                                            target_entity,
                                            Some(source_entity),
                                            world,
                                        );
                                        return Err(InputParseError::CommandParseError {
                                            verb: THROW_VERB_NAME.to_string(),
                                            error: CommandParseError::Other(format!(
                                                "You can't throw anything at {target_name}."
                                            )),
                                        });
                                    }
                                }
                            }
                        } else {
                            // target doesn't exist
                            return Err(InputParseError::CommandParseError {
                                verb: THROW_VERB_NAME.to_string(),
                                error: CommandParseError::TargetNotFound(target),
                            });
                        }
                    } else {
                        // item to throw doesn't exist
                        return Err(InputParseError::CommandParseError {
                            verb: THROW_VERB_NAME.to_string(),
                            error: CommandParseError::TargetNotFound(item),
                        });
                    }
                } else {
                    // target wasn't provided
                    return Err(InputParseError::CommandParseError {
                        verb: THROW_VERB_NAME.to_string(),
                        error: CommandParseError::MissingTarget,
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![THROW_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<Item>(entity, world, &[THROW_FORMAT])
    }
}

enum CannotThrowReason {
    NotThrowable,
    TooWeak,
}

/// Determines if there's anything preventing the provided entity from throwing the provided item.
///
/// Returns `None` if the entity can throw the item, `Some` otherwise.
fn get_cannot_throw_reason(
    thrower: Entity,
    item: Entity,
    world: &World,
) -> Option<CannotThrowReason> {
    if world.get::<Item>(item).is_none() {
        // only items can be thrown
        return Some(CannotThrowReason::NotThrowable);
    }

    let item_weight = get_weight(item, world);

    let max_weight_can_throw = if let Some(stats) = world.get::<Stats>(thrower) {
        let strength = stats.attributes.get(Attribute::Strength);
        Weight(strength as f32 * KG_CAN_THROW_PER_STRENGTH)
    } else {
        // the thrower has no strength, so can only throw things with no weight of course
        Weight(0.0)
    };

    if item_weight > max_weight_can_throw {
        return Some(CannotThrowReason::TooWeak);
    }

    None
}

#[derive(Debug)]
pub struct ThrowAction {
    /// The item to throw
    pub item: Entity,
    /// The entity to throw the item at
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for ThrowAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let item = self.item;
        let target = self.target;
        let item_name = get_reference_name(item, Some(performing_entity), world);
        let target_name = get_reference_name(target, Some(performing_entity), world);
        let current_location_id = world
            .get::<Location>(performing_entity)
            .expect("Throwing entity should have a location")
            .id;

        // determine how hard it'll be to throw the item at the target

        // if the target is alive, it will try to not get hit

        //TODO

        // larger items are easier to hit things with, but also harder to throw, so let's say that cancels out and so the only relevant things are the weight of the item being thrown and the size of the target

        let target_volume = get_volume(target, world);
        let item_weight = get_weight(item, world);
        let base_difficulty = MIN_BASE_DIFFICULTY.max(item_weight.0 * DIFFICULTY_PER_KG);
        let target_volume_multiplier = if target_volume.0 > 0.0 {
            (1.0 / target_volume.0) * VOLUME_DIFFICULTY_MULT_MULT
        } else {
            MAX_VOLUME_DIFFICULTY_MULT
        }
        .clamp(MIN_VOLUME_DIFFICULTY_MULT, MAX_VOLUME_DIFFICULTY_MULT);
        let check_target = base_difficulty * target_volume_multiplier;

        let mut result_builder = ActionResult::builder();

        match Stats::check_attribute(
            performing_entity,
            Attribute::Strength,
            CheckDifficulty::new(check_target.round() as u32),
            world,
        ) {
            CheckResult::ExtremeFailure => {
                result_builder = result_builder
                    .with_message(
                        performing_entity,
                        format!(
                        "You hurl {item_name} wildly, and it comes nowhere close to {target_name}."
                    ),
                        MessageCategory::Internal(InternalMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .with_third_person_message(
                        Some(performing_entity),
                        ThirdPersonMessageLocation::SourceEntity,
                        ThirdPersonMessage::new(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                            MessageDelay::Short,
                        )
                        .add_entity_name(performing_entity)
                        .add_string(" hurls ")
                        .add_entity_name(item)
                        .add_string(" wildly, and it comes nowhere close to ")
                        .add_entity_name(target)
                        .add_string("."),
                        world,
                    )
            }
            CheckResult::Failure => {
                result_builder = result_builder
                    .with_message(
                        performing_entity,
                        format!("You throw {item_name}, and it just misses {target_name}."),
                        MessageCategory::Internal(InternalMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .with_third_person_message(
                        Some(performing_entity),
                        ThirdPersonMessageLocation::SourceEntity,
                        ThirdPersonMessage::new(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                            MessageDelay::Short,
                        )
                        .add_entity_name(performing_entity)
                        .add_string(" throws ")
                        .add_entity_name(item)
                        .add_string(", and it just misses ")
                        .add_entity_name(target)
                        .add_string("."),
                        world,
                    )
            }
            CheckResult::Success => {
                result_builder = result_builder
                    .with_message(
                        performing_entity,
                        format!("You throw {item_name}, and it hits {target_name}."),
                        MessageCategory::Internal(InternalMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .with_third_person_message(
                        Some(performing_entity),
                        ThirdPersonMessageLocation::SourceEntity,
                        ThirdPersonMessage::new(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                            MessageDelay::Short,
                        )
                        .add_entity_name(performing_entity)
                        .add_string(" throws ")
                        .add_entity_name(item)
                        .add_string(", and it hits ")
                        .add_entity_name(target)
                        .add_string("."),
                        world,
                    );
                if is_living_entity(target, world) {
                    let damage = item_weight.0 * HIT_DAMAGE_PER_KG;
                    let item_reference_name = get_article_reference_name(item, world);
                    result_builder = result_builder.with_post_effect(Box::new(move |w| {
                        ValueChange {
                            entity: target,
                            value_type: ValueType::Health,
                            operation: ValueChangeOperation::Subtract,
                            amount: damage,
                            message: Some(format!("Ow, you got hit with {item_reference_name}!")),
                        }
                        .apply(w)
                    }))
                }
            }
            CheckResult::ExtremeSuccess => {
                result_builder = result_builder
                    .with_message(
                        performing_entity,
                        format!(
                            "You deftly throw {item_name}, and it impacts {target_name} perfectly."
                        ),
                        MessageCategory::Internal(InternalMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .with_third_person_message(
                        Some(performing_entity),
                        ThirdPersonMessageLocation::SourceEntity,
                        ThirdPersonMessage::new(
                            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                            MessageDelay::Short,
                        )
                        .add_entity_name(performing_entity)
                        .add_string(" deftly throws ")
                        .add_entity_name(item)
                        .add_string(", and it impacts ")
                        .add_entity_name(target)
                        .add_string(" perfectly."),
                        world,
                    );
                if is_living_entity(target, world) {
                    let damage = item_weight.0 * HIT_DAMAGE_PER_KG * DIRECT_HIT_DAMAGE_MULT;
                    let item_reference_name = get_article_reference_name(item, world);
                    result_builder = result_builder.with_post_effect(Box::new(move |w| {
                        ValueChange {
                            entity: target,
                            value_type: ValueType::Health,
                            operation: ValueChangeOperation::Subtract,
                            amount: damage,
                            message: Some(format!("Ow, you got hit with {item_reference_name}!")),
                        }
                        .apply(w)
                    }))
                }
            }
        }

        // move the item to the room
        result_builder = result_builder.with_post_effect(Box::new(move |w| {
            move_entity(item, current_location_id, w);
        }));

        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop throwing.".to_string(),
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

//TODO auto-equip the item to throw if it isn't equipped

//TODO validate that the item to throw is equipped
