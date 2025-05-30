use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use bevy_ecs::prelude::*;

use crate::{
    checks::{CheckDifficulty, CheckModifiers, CheckResult, VsCheckParams, VsParticipant},
    command_format::{
        entity_part_with_validator, literal_part, validate_parsed_value_has_component,
        CommandFormat, CommandParseError, CommandPartId, CommandPartValidateError,
        CommandPartValidateResult, PartValidatorContext,
    },
    component::{
        ActionEndNotification, ActionQueue, AfterActionPerformNotification, Attribute, CombatRange,
        EquippedItems, Item, Location, Skill, Stats, Weight,
    },
    handle_enter_combat,
    input_parser::{input_formats_if_has_component, InputParser},
    is_living_entity, move_entity,
    notification::{Notification, VerifyResult},
    vital_change::{
        ValueChangeOperation, VitalChange, VitalChangeMessageParams, VitalChangeVisualizationType,
        VitalType,
    },
    ActionTag, BeforeActionNotification, Description, DynamicMessage, DynamicMessageLocation,
    GameMessage, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    MessageTokens, SurroundingsMessageCategory, TokenName, TokenValue, VerifyActionNotification,
    Volume, Xp, STANDARD_CHECK_XP,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult, EquipAction};

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

static ITEM_PART_ID: LazyLock<CommandPartId<Entity>> = LazyLock::new(|| CommandPartId::new("item"));
static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static THROW_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("throw"))
        .then(literal_part(" "))
        .then(
            entity_part_with_validator(ITEM_PART_ID.clone(), |context, world| {
                validate_parsed_value_has_component::<Item>(context, "throw", world)
            })
            .with_if_missing("what")
            .with_placeholder_for_format_string("thing"),
        )
        .then(literal_part(" at "))
        .then(
            entity_part_with_validator(TARGET_PART_ID.clone(), validate_target)
                .with_if_missing("what")
                .with_placeholder_for_format_string("target"),
        )
});

/// Checks that an entity can have things thrown at it.
fn validate_target(
    context: PartValidatorContext<Entity>,
    world: &World,
) -> CommandPartValidateResult {
    if world.get::<Item>(context.parsed_value).is_some()
        || is_living_entity(context.parsed_value, world)
    {
        return CommandPartValidateResult::Valid;
    }

    let target_name = Description::get_reference_name(
        context.parsed_value,
        Some(context.performing_entity),
        world,
    );
    CommandPartValidateResult::Invalid(CommandPartValidateError {
        details: Some(format!("You can't throw anything at {target_name}.")),
    })
}

pub struct ThrowParser;

impl InputParser for ThrowParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = THROW_FORMAT.parse(input, source_entity, world)?;
        let item = parsed.get(&ITEM_PART_ID);
        let target = parsed.get(&TARGET_PART_ID);

        if target == source_entity {
            return Err(CommandParseError::Other(
                "You can't throw things at yourself.".to_string(),
            ));
        }

        if item == target {
            let item_name = Description::get_reference_name(item, Some(source_entity), world);
            return Err(CommandParseError::Other(format!(
                "You can't throw {item_name} at itself."
            )));
        }

        Ok(Box::new(ThrowAction {
            item,
            target,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![THROW_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        if is_living_entity(entity, world) {
            return vec![THROW_FORMAT
                .get_format_description()
                .with_targeted_entity(TARGET_PART_ID.clone(), entity, world)
                .to_string()];
        }

        input_formats_if_has_component::<Item>(
            entity,
            world,
            &[
                THROW_FORMAT.get_format_description().with_targeted_entity(
                    ITEM_PART_ID.clone(),
                    entity,
                    world,
                ),
                THROW_FORMAT.get_format_description().with_targeted_entity(
                    TARGET_PART_ID.clone(),
                    entity,
                    world,
                ),
            ],
        )
    }
}

/// Determines if the provided entity is strong enough to throw the provided item.
fn is_strong_enough_to_throw(thrower: Entity, item: Entity, world: &World) -> bool {
    let item_weight = Weight::get(item, world);

    let max_weight_can_throw = if let Some(stats) = world.get::<Stats>(thrower) {
        let strength = stats.attributes.get(&Attribute::Strength);
        Weight(strength as f32 * KG_CAN_THROW_PER_STRENGTH)
    } else {
        // the thrower has no strength, so can only throw things with no weight of course
        Weight(0.0)
    };

    item_weight <= max_weight_can_throw
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
                VsCheckParams::second_wins_ties(STANDARD_CHECK_XP),
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
                Xp(0), // you don't get XP for just throwing stuff at inanimate objects
                world,
            );
        }

        let tokens = ThrowMessageTokens {
            thrower: performing_entity,
            item,
            target,
        };

        let hit;
        let dynamic_message = match throw_result {
            CheckResult::ExtremeFailure => {
                hit = false;
                if target_is_living {
                    build_dodge_extreme_success_message(tokens)
                } else {
                    build_throw_extreme_fail_message(tokens)
                }
            }
            CheckResult::Failure => {
                hit = false;
                if target_is_living {
                    build_dodge_success_message(tokens)
                } else {
                    build_throw_fail_message(tokens)
                }
            }
            CheckResult::Success => {
                hit = true;
                if target_is_living {
                    build_dodge_fail_message(tokens)
                } else {
                    build_throw_success_message(tokens)
                }
            }
            CheckResult::ExtremeSuccess => {
                hit = true;
                if target_is_living {
                    build_dodge_extreme_fail_message(tokens)
                } else {
                    build_throw_extreme_success_message(tokens)
                }
            }
        };

        if hit && target_is_living {
            let mut damage = get_hit_damage(item, world);
            if CheckResult::ExtremeSuccess == throw_result {
                damage *= DIRECT_HIT_DAMAGE_MULT;
            }
            let item_reference_name = Description::get_article_reference_name(item, world);
            result_builder = result_builder.with_post_effect(Box::new(move |w| {
                VitalChange::<ThrowMessageTokens> {
                    entity: target,
                    vital_type: VitalType::Health,
                    operation: ValueChangeOperation::Subtract,
                    amount: damage,
                    message_params: vec![
                        (
                            VitalChangeMessageParams::Dynamic(dynamic_message),
                            VitalChangeVisualizationType::Abbreviated,
                        ),
                        (
                            VitalChangeMessageParams::Direct {
                                entity: target,
                                message: format!("Ow, you got hit with {item_reference_name}!"),
                                category: MessageCategory::Internal(InternalMessageCategory::Misc),
                            },
                            VitalChangeVisualizationType::Full,
                        ),
                    ],
                }
                .apply(w)
            }))
        } else {
            result_builder = result_builder.with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                dynamic_message,
                world,
            );
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

/// Tokens for messages about throws.
struct ThrowMessageTokens {
    /// The entity doing the throwing
    thrower: Entity,
    /// The entity getting thrown
    item: Entity,
    /// The entity getting thrown at
    target: Entity,
}

impl MessageTokens for ThrowMessageTokens {
    fn get_token_map(&self) -> HashMap<TokenName, TokenValue> {
        [
            ("thrower".into(), TokenValue::Entity(self.thrower)),
            ("item".into(), TokenValue::Entity(self.item)),
            ("target".into(), TokenValue::Entity(self.target)),
        ]
        .into()
    }
}

/// Builds a message for when the throw was an extreme failure.
fn build_throw_extreme_fail_message(
    tokens: ThrowMessageTokens,
) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:hurl/hurls} ${item.name} wildly, and it comes nowhere close to ${target.name}.")
            .expect("message format should be valid"),
        tokens,
    )
}

/// Builds a message for when the throw was a failure.
fn build_throw_fail_message(tokens: ThrowMessageTokens) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new(
            "${thrower.Name} ${thrower.you:throw/throws} ${item.name}, and ${item.they} ${item.whiz/whizzes} just past ${target.name}.",
        )
        .expect("message format should be valid"),
        tokens,
    )
}

/// Builds a message for when the dodge was an extreme failure.
fn build_dodge_extreme_fail_message(
    tokens: ThrowMessageTokens,
) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:throw/throws} ${item.name}, and it seems like ${target.name} ${target.you:don't/doesn't} even try to move out of the way before ${item.they} ${item.hit/hits} ${target.them} directly in the face.")
            .expect("message format should be valid"),
            tokens,
    )
}

/// Builds a message for when the dodge was a failure.
fn build_dodge_fail_message(tokens: ThrowMessageTokens) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:throw/throws} ${item.name}, and ${target.name} ${target.you:aren't/isn't} able to get out of the way before ${item.they} ${item.hit/hits} ${target.them} in the chest.")
            .expect("message format should be valid"),
            tokens,
    )
}

/// Builds a message for when the dodge was a success.
fn build_dodge_success_message(tokens: ThrowMessageTokens) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:throw/throws} ${item.name}, but ${target.name} ${target.you:move/moves} out of the way just before ${item.they} ${item.hit/hits} ${target.them}.")
            .expect("message format should be valid"),
            tokens,
    )
}

/// Builds a message for when the dodge was an extreme success.
fn build_dodge_extreme_success_message(
    tokens: ThrowMessageTokens,
) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:throw/throws} ${item.name}, but ${target.name} calmly ${target.you:shift/shifts} just enough to avoid being hit.")
            .expect("message format should be valid"),
            tokens,
    )
}

/// Builds a message for when the throw was a success and the target didn't attempt to dodge.
fn build_throw_success_message(tokens: ThrowMessageTokens) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} ${thrower.you:throw/throws} ${item.name}, and ${item.they} ${item.hit/hits} ${target.name}.")
            .expect("message format should be valid"),
            tokens,
    )
}

/// Builds a message for when the throw was an extreme success and the target didn't attempt to dodge.
fn build_throw_extreme_success_message(
    tokens: ThrowMessageTokens,
) -> DynamicMessage<ThrowMessageTokens> {
    DynamicMessage::new(
        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
        MessageDelay::Short,
        MessageFormat::<ThrowMessageTokens>::new("${thrower.Name} deftly ${thrower.you:throw/throws} ${item.name}, and ${item.they} ${item.impact/impacts} ${target.name} perfectly.")
            .expect("message format should be valid"),
            tokens,
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
    if Some(performing_entity) == world.get::<Location>(item).map(|loc| loc.id)
        && !EquippedItems::is_equipped(performing_entity, item, world)
    {
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

/// Verifies that the entity trying to throw an item has it equipped.
pub fn verify_wielding_item_to_throw(
    notification: &Notification<VerifyActionNotification, ThrowAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;

    if EquippedItems::is_equipped(performing_entity, item, world) {
        return VerifyResult::valid();
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

/// Verifies that the thrower is strong enough to throw the thing they're trying to throw.
/// TODO register this
pub fn verify_strong_enough_to_throw_item(
    notification: &Notification<VerifyActionNotification, ThrowAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.item;
    let performing_entity = notification.notification_type.performing_entity;

    if is_strong_enough_to_throw(performing_entity, item, world) {
        return VerifyResult::valid();
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);
    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You're not strong enough to throw {item_name}.")),
    )
}
