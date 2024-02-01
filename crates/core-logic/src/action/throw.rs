use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{ThirdPersonMessage, ThirdPersonMessageLocation},
    checks::{CheckDifficulty, CheckModifiers, CheckResult, VsCheckParams, VsParticipant},
    component::{
        ActionEndNotification, ActionQueue, AfterActionPerformNotification, Attribute, CombatRange,
        EquippedItems, Item, Location, Skill, Stats, Weight,
    },
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    is_living_entity, move_entity,
    notification::{Notification, VerifyResult},
    vital_change::{ValueChangeOperation, VitalChange, VitalType},
    BeforeActionNotification, Description, GameMessage, InternalMessageCategory, MessageCategory,
    MessageDelay, Pronouns, SurroundingsMessageCategory, VerifyActionNotification, Volume,
};

use super::{
    handle_enter_combat, Action, ActionInterruptResult, ActionNotificationSender, ActionResult,
    ActionResultBuilder, EquipAction,
};

/// The number of kilograms an entity can throw per point of strength they have.
const KG_CAN_THROW_PER_STRENGTH: f32 = 2.0;

/// The amount of damage to do to an entity that is hit by a thrown object per kilogram the thrown object weighs.
const HIT_DAMAGE_PER_KG: f32 = 3.0;

/// The amount to multiply the hit damage by if it's a really good throw.
const DIRECT_HIT_DAMAGE_MULT: f32 = 2.0;

/// The amount to multiply the difficulty multiplier of a throw check by for hitting an inanimate object with a volume of 1 liter
const VOLUME_DIFFICULTY_MULT_MULT: f32 = 3.0;

/// The penalty applied to throw checks per kilogram the thrown object weighs
const WEIGHT_PENALTY_PER_KG: f32 = 0.5;

/// The base difficulty of throw checks against inanimate objects
const BASE_DIFFICULTY: f32 = 5.0;

/// The minimum amount to multiply throw check difficulty by due to the size of the target
const MIN_VOLUME_DIFFICULTY_MULT: f32 = 0.5;

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
                                let item_name = Description::get_reference_name(
                                    item_entity,
                                    Some(source_entity),
                                    world,
                                );
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
                                    let item_name = Description::get_reference_name(
                                        item_entity,
                                        Some(source_entity),
                                        world,
                                    );
                                    return Err(InputParseError::CommandParseError {
                                        verb: THROW_VERB_NAME.to_string(),
                                        error: CommandParseError::Other(format!(
                                            "You can't throw {item_name}."
                                        )),
                                    });
                                }
                                Some(CannotThrowReason::TooWeak) => {
                                    let item_name = Description::get_reference_name(
                                        item_entity,
                                        Some(source_entity),
                                        world,
                                    );
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
                                        let target_name = Description::get_reference_name(
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

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
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

    let item_weight = Weight::get(item, world);

    let max_weight_can_throw = if let Some(stats) = world.get::<Stats>(thrower) {
        let strength = stats.attributes.get(&Attribute::Strength);
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

/// Makes an entity throw an item.
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
        let current_location_id = world
            .get::<Location>(performing_entity)
            .expect("Throwing entity should have a location")
            .id;
        let target_is_living = is_living_entity(target, world);

        let mut result_builder = ActionResult::builder();

        if target_is_living {
            result_builder = handle_enter_combat(
                performing_entity,
                target,
                CombatRange::Long,
                result_builder,
                world,
            );
        }

        let throw_penalty = get_throw_penalty(item, world);

        let throw_result;
        if target_is_living {
            // the target is alive, so it will try to dodge
            (throw_result, _) = Stats::check_vs(
                VsParticipant {
                    entity: performing_entity,
                    stat: Attribute::Strength.into(),
                    modifiers: CheckModifiers::modify_value(-throw_penalty),
                },
                VsParticipant {
                    entity: performing_entity,
                    stat: Skill::Dodge.into(),
                    modifiers: CheckModifiers::none(),
                },
                VsCheckParams::second_wins_ties(),
                world,
            );
        } else {
            // the taget is not alive, so the difficulty of the throw is modified by the size of the target
            let difficulty = get_inanimate_target_difficulty(target, world);
            throw_result = Stats::check(
                performing_entity,
                Attribute::Strength,
                CheckModifiers::modify_value(-throw_penalty),
                difficulty,
                world,
            );
        }

        let message_context = ThrowMessageContext {
            performing_entity,
            item,
            item_name: Description::get_reference_name(item, Some(performing_entity), world),
            target,
            target_name: Description::get_reference_name(target, Some(performing_entity), world),
            target_pronoun: Pronouns::get_personal_object(target, world),
        };

        let hit;
        match throw_result {
            CheckResult::ExtremeFailure => {
                hit = false;
                if target_is_living {
                    result_builder = result_builder_with_dodge_extreme_success_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                } else {
                    result_builder = result_builder_with_throw_extreme_fail_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                }
            }
            CheckResult::Failure => {
                hit = false;
                if target_is_living {
                    result_builder = result_builder_with_dodge_success_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                } else {
                    result_builder = result_builder_with_throw_fail_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                }
            }
            CheckResult::Success => {
                hit = true;
                if target_is_living {
                    result_builder = result_builder_with_dodge_fail_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                } else {
                    result_builder = result_builder_with_throw_success_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                }
            }
            CheckResult::ExtremeSuccess => {
                hit = true;
                if target_is_living {
                    result_builder = result_builder_with_dodge_extreme_fail_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                } else {
                    result_builder = result_builder_with_throw_extreme_success_messages(
                        result_builder,
                        &message_context,
                        world,
                    );
                }
            }
        }

        if hit && target_is_living {
            let mut damage = get_hit_damage(item, world);
            if CheckResult::ExtremeSuccess == throw_result {
                damage *= DIRECT_HIT_DAMAGE_MULT;
            }
            let item_reference_name = Description::get_article_reference_name(item, world);
            result_builder = result_builder.with_post_effect(Box::new(move |w| {
                VitalChange {
                    entity: target,
                    vital_type: VitalType::Health,
                    operation: ValueChangeOperation::Subtract,
                    amount: damage,
                    message: Some(format!("Ow, you got hit with {item_reference_name}!")),
                }
                .apply(w)
            }))
        }

        // unequip the item and move it to the room
        result_builder = result_builder.with_post_effect(Box::new(move |w| {
            EquippedItems::unequip(performing_entity, item, w)
                .expect("Should be able to unequip thrown item");
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

/// Determines the throw check penalty for throwing the provided item
fn get_throw_penalty(item: Entity, world: &World) -> f32 {
    // larger items are easier to hit things with, but also harder to throw, so let's say that cancels out and so the only relevant thing is the weight of the item being thrown
    Weight::get(item, world).0 * WEIGHT_PENALTY_PER_KG
}

/// Determines how much damage the provided entity does when it hits something
fn get_hit_damage(item: Entity, world: &World) -> f32 {
    Weight::get(item, world).0 * HIT_DAMAGE_PER_KG
}

/// Determines the difficulty of a throw check targeting the provided inanimate object.
fn get_inanimate_target_difficulty(target: Entity, world: &World) -> CheckDifficulty {
    let target_volume = Volume::get(target, world);
    let target_volume_multiplier = if target_volume.0 > 0.0 {
        (1.0 / target_volume.0) * VOLUME_DIFFICULTY_MULT_MULT
    } else {
        MAX_VOLUME_DIFFICULTY_MULT
    }
    .clamp(MIN_VOLUME_DIFFICULTY_MULT, MAX_VOLUME_DIFFICULTY_MULT);
    let check_target = BASE_DIFFICULTY * target_volume_multiplier;

    CheckDifficulty::new(check_target.round() as u16)
}

/// Context for building messages about throws.
struct ThrowMessageContext {
    /// The entity doing the throwing.
    performing_entity: Entity,
    /// The item being thrown.
    item: Entity,
    /// The name of the item being thrown (including "the").
    item_name: String,
    /// The entity being thrown at.
    target: Entity,
    /// The name of the entity being thrown at (including "the", if necessary).
    target_name: String,
    /// The personal object pronoun of the entity being thrown at (e.g. him, her, them).
    target_pronoun: String,
}

/// Adds messages to the provided result builder for when the throw was an extreme failure.
fn result_builder_with_throw_extreme_fail_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;

    result_builder
        .with_message(
            context.performing_entity,
            format!("You hurl {item_name} wildly, and it comes nowhere close to {target_name}."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(context.performing_entity)
            .add_string(" hurls ")
            .add_name(context.item)
            .add_string(" wildly, and it comes nowhere close to ")
            .add_name(context.target)
            .add_string("."),
            world,
        )
}

/// Adds messages to the provided result builder for when the throw was a failure.
fn result_builder_with_throw_fail_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;

    result_builder
        .with_message(
            context.performing_entity,
            format!("You throw {item_name}, and it whizzes just past {target_name}."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(context.performing_entity)
            .add_string(" throws ")
            .add_name(context.item)
            .add_string(", and it whizzes just past ")
            .add_name(context.target)
            .add_string("."),
            world,
        )
}

/// Adds messages to the provided result builder for when the dodge was an extreme failure.
fn result_builder_with_dodge_extreme_fail_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;
    let target_pronoun = &context.target_pronoun;

    let message = format!("You throw {item_name}, and it seems like {target_name} doesn't even try to move out of the way before it hits {target_pronoun} directly in the face.");

    let target_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .only_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", and it seems like you don't even try to move out of the way before it hits you directly in the face.");

    let third_person_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .do_not_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", and it seems like ")
    .add_name(context.target)
    .add_string(" doesn't even try to move out of the way before it hits ")
    .add_personal_object_pronoun(context.target)
    .add_string(" directly in the face.");

    result_builder
        .with_message(
            context.performing_entity,
            message,
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            target_message,
            world,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            third_person_message,
            world,
        )
}

/// Adds messages to the provided result builder for when the dodge was a failure.
fn result_builder_with_dodge_fail_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;
    let target_pronoun = &context.target_pronoun;

    let message = format!("You throw {item_name}, and {target_name} isn't able to get out of the way before it hits {target_pronoun} in the chest.");

    let target_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .only_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", and you aren't able to get out of the way before it hits you in the chest.");

    let third_person_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .do_not_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", and ")
    .add_name(context.target)
    .add_string(" isn't able to get out of the way before it hits ")
    .add_personal_object_pronoun(context.target)
    .add_string(" in the chest.");

    result_builder
        .with_message(
            context.performing_entity,
            message,
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            target_message,
            world,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            third_person_message,
            world,
        )
}

/// Adds messages to the provided result builder for when the dodge was a success.
fn result_builder_with_dodge_success_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;
    let target_pronoun = &context.target_pronoun;

    let message = format!("You throw {item_name}, but {target_name} moves out of the way just before it hits {target_pronoun}.");

    let target_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .only_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", but you move out of the way just before it hits you.");

    let third_person_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .do_not_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", but ")
    .add_name(context.target)
    .add_string(" moves out of the way just before it hits ")
    .add_personal_object_pronoun(context.target)
    .add_string(".");

    result_builder
        .with_message(
            context.performing_entity,
            message,
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            target_message,
            world,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            third_person_message,
            world,
        )
}

/// Adds messages to the provided result builder for when the dodge was an extreme success.
fn result_builder_with_dodge_extreme_success_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;

    let message = format!(
        "You throw {item_name}, but {target_name} calmly shifts just enough to avoid being hit."
    );

    let target_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .only_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", but you calmly shift just enough to avoid being hit.");

    let third_person_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .do_not_send_to(context.target)
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", but ")
    .add_name(context.target)
    .add_string(" calmly shifts just enough to avoid being hit.");

    result_builder
        .with_message(
            context.performing_entity,
            message,
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            target_message,
            world,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            third_person_message,
            world,
        )
}

/// Adds messages to the provided result builder for when the throw was a success and the target didn't attempt to dodge.
fn result_builder_with_throw_success_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;

    let message = format!("You throw {item_name}, and it hits {target_name}.");
    let third_person_message = ThirdPersonMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
    )
    .add_name(context.performing_entity)
    .add_string(" throws ")
    .add_name(context.item)
    .add_string(", and it hits ")
    .add_name(context.target)
    .add_string(".");

    result_builder
        .with_message(
            context.performing_entity,
            message,
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            third_person_message,
            world,
        )
}

/// Adds messages to the provided result builder for when the throw was an extreme success and the target didn't attempt to dodge.
fn result_builder_with_throw_extreme_success_messages(
    result_builder: ActionResultBuilder,
    context: &ThrowMessageContext,
    world: &World,
) -> ActionResultBuilder {
    let item_name = &context.item_name;
    let target_name = &context.target_name;

    result_builder
        .with_message(
            context.performing_entity,
            format!("You deftly throw {item_name}, and it impacts {target_name} perfectly."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(context.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(context.performing_entity)
            .add_string(" deftly throws ")
            .add_name(context.item)
            .add_string(", and it impacts ")
            .add_name(context.target)
            .add_string(" perfectly."),
            world,
        )
}

/// Attempts to equip the item to throw automatically before an attempt is made to throw it.
pub fn auto_equip_item_to_throw(
    notification: &Notification<BeforeActionNotification, ThrowAction>,
    world: &mut World,
) {
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;

    // only try to equip the item if the thrower already has it, since otherwise the equip action will just fail anyway
    if Some(performing_entity) == world.get::<Location>(item).map(|loc| loc.id) {
        if let Some(equipped_items) = world.get::<EquippedItems>(performing_entity) {
            if !equipped_items.is_equipped(item) {
                ActionQueue::queue_first(
                    world,
                    notification.notification_type.performing_entity,
                    Box::new(EquipAction {
                        target: item,
                        should_be_equipped: true,
                        notification_sender: ActionNotificationSender::new(),
                    }),
                );
            }
        }
    }
}

/// Verifies that the entity trying to throw an item has it equipped.
pub fn verify_wielding_item_to_throw(
    notification: &Notification<VerifyActionNotification, ThrowAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(eqipped_items) = world.get::<EquippedItems>(performing_entity) {
        if eqipped_items.is_equipped(item) {
            return VerifyResult::valid();
        }
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You don't have {item_name} equipped.")),
    )
}

/// Verifies that the target is in the same room as the thrower.
pub fn verify_target_in_same_room(
    notification: &Notification<VerifyActionNotification, ThrowAction>,
    world: &World,
) -> VerifyResult {
    let target = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(thrower_location) = world.get::<Location>(performing_entity) {
        if Some(thrower_location) == world.get::<Location>(target) {
            return VerifyResult::valid();
        }
    }

    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("{target_name} isn't here.")),
    )
}
