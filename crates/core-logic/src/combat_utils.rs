use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;
use rand::seq::SliceRandom;
use regex::{Captures, Regex};

use crate::{
    body_part::BodyPartDamageMultiplier,
    is_living_entity,
    resource::WeaponTypeStatCatalog,
    vital_change::{ValueChangeOperation, VitalChangeMessageParams, VitalChangeVisualizationType},
    Action, ActionNotificationSender, ActionQueue, ActionResult, ActionResultBuilder, ActionTag,
    AttackType, BasicTokens, BeforeActionNotification, BodyPart, CheckModifiers, CheckResult,
    CombatRange, CombatState, CommandParseError, CommandTarget, Container, Description,
    DynamicMessage, DynamicMessageLocation, EquipAction, EquippedItems, ExitCombatNotification,
    GameMessage, InnateWeapon, InputParseError, IntegerExtensions, InternalMessageCategory,
    Location, MessageCategory, MessageDelay, MessageFormat, Notification, Skill, Stats,
    SurroundingsMessageCategory, VerifyActionNotification, VerifyResult, VitalChange, VitalType,
    Vitals, VsCheckParams, VsParticipant, Weapon, WeaponHitMessageTokens, WeaponMissMessageTokens,
    WeaponUnusableError, STANDARD_CHECK_XP,
};

/// The fraction of a target's health that counts as a high amount of damage.
const HIGH_DAMAGE_THRESHOLD: f32 = 0.4;

/// The fraction of a target's health that counts as a low amount of damage.
const LOW_DAMAGE_THRESHOLD: f32 = 0.1;

/// Describes an attack, parsed into entities.
pub struct ParsedAttack {
    /// The target of the attack.
    pub target: Entity,
    /// The weapon to use for the attack.
    pub weapon: ChosenWeapon,
}

/// Represents the weapon chosen for an attack.
#[derive(Debug, Clone, Copy)]
pub enum ChosenWeapon {
    /// A specific weapon
    Entity(Entity),
    /// Whatever weapon the entity happens to be holding at the time
    Unspecified,
}

impl ChosenWeapon {
    /// Finds the entity for the chosen weapon to be used by `attacking_entity`, if one can be found.
    pub fn get_entity<A: AttackType>(
        &self,
        attacking_entity: Entity,
        world: &World,
    ) -> Option<Entity> {
        match self {
            ChosenWeapon::Entity(e) => Some(*e),
            ChosenWeapon::Unspecified => choose_weapon::<A>(attacking_entity, world),
        }
    }
}

/// Chooses a weapon for the provided entity to attack with.
fn choose_weapon<A: AttackType>(attacking_entity: Entity, world: &World) -> Option<Entity> {
    // prioritize the primary weapon
    if let Some((_, weapon_entity)) = Weapon::get_primary(attacking_entity, world) {
        if A::can_perform_with(weapon_entity, world) {
            return Some(weapon_entity);
        }
    }

    // primary weapon didn't match, so fall back to other equipped weapons
    if let Some(equipped_items) = world.get::<EquippedItems>(attacking_entity) {
        for item in equipped_items.get_items() {
            if world.get::<Weapon>(*item).is_some() && A::can_perform_with(*item, world) {
                return Some(*item);
            }
        }
    }

    // no equipped weapons matched, try innate weapon
    if let Some((_, innate_weapon_entity)) = InnateWeapon::get(attacking_entity, world) {
        if A::can_perform_with(innate_weapon_entity, world) {
            return Some(innate_weapon_entity);
        }
    }

    // no equipped weapons or innate weapon matched, fall back to non-equipped weapons
    if let Some(container) = world.get::<Container>(attacking_entity) {
        for item in container.get_entities(attacking_entity, world) {
            if world.get::<Weapon>(item).is_some() && A::can_perform_with(item, world) {
                return Some(item);
            }
        }
    }

    // couldn't find a matching weapon
    None
}

/// Parses input from `source_entity` as an attack command.
/// `pattern` should have a capture group with the name provided in `target_capture_name`. Any other capture groups will be ignored.
///
/// Returns `Ok` with the target entity, or `Err` if the input is invalid.
pub fn parse_attack_input<A: AttackType>(
    input: &str,
    source_entity: Entity,
    pattern: &Regex,
    pattern_with_weapon: &Regex,
    target_capture_name: &str,
    weapon_capture_name: &str,
    verb_name: &str,
    world: &World,
) -> Result<ParsedAttack, InputParseError> {
    if let Some(captures) = pattern_with_weapon.captures(input) {
        return parse_attack_input_captures::<A>(
            &captures,
            source_entity,
            target_capture_name,
            weapon_capture_name,
            verb_name,
            world,
        );
    }

    if let Some(captures) = pattern.captures(input) {
        return parse_attack_input_captures::<A>(
            &captures,
            source_entity,
            target_capture_name,
            weapon_capture_name,
            verb_name,
            world,
        );
    }

    Err(InputParseError::UnknownCommand)
}

fn parse_attack_input_captures<A: AttackType>(
    captures: &Captures,
    source_entity: Entity,
    target_capture_name: &str,
    weapon_capture_name: &str,
    verb_name: &str,
    world: &World,
) -> Result<ParsedAttack, InputParseError> {
    let target_entity = parse_attack_target(
        captures,
        target_capture_name,
        source_entity,
        verb_name,
        world,
    )?;
    let chosen_weapon = parse_attack_weapon::<A>(
        captures,
        weapon_capture_name,
        source_entity,
        verb_name,
        world,
    )?;

    Ok(ParsedAttack {
        target: target_entity,
        weapon: chosen_weapon,
    })
}

/// Finds the target entity of an attack.
fn parse_attack_target(
    captures: &Captures,
    target_capture_name: &str,
    source_entity: Entity,
    verb_name: &str,
    world: &World,
) -> Result<Entity, InputParseError> {
    if let Some(target_match) = captures.name(target_capture_name) {
        let target = CommandTarget::parse(target_match.as_str());
        if let Some(target_entity) = target.find_target_entity(source_entity, world) {
            if world.get::<Vitals>(target_entity).is_some() {
                // target exists and is attackable
                return Ok(target_entity);
            }
            let target_name =
                Description::get_reference_name(target_entity, Some(source_entity), world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::Other(format!("You can't attack {target_name}.")),
            });
        }
        return Err(InputParseError::CommandParseError {
            verb: verb_name.to_string(),
            error: CommandParseError::TargetNotFound(target),
        });
    }

    // no target provided
    let combatants = CombatState::get_entities_in_combat_with(source_entity, world);
    if combatants.len() == 1 {
        let target_entity = combatants
            .keys()
            .next()
            .expect("combatants should contain an entry");
        return Ok(*target_entity);
    }

    Err(InputParseError::CommandParseError {
        verb: verb_name.to_string(),
        error: CommandParseError::MissingTarget,
    })
}

/// Finds the weapon entity to use in an attack.
fn parse_attack_weapon<A: AttackType>(
    captures: &Captures,
    weapon_capture_name: &str,
    source_entity: Entity,
    verb_name: &str,
    world: &World,
) -> Result<ChosenWeapon, InputParseError> {
    if let Some(target_match) = captures.name(weapon_capture_name) {
        let weapon = CommandTarget::parse(target_match.as_str());
        if let Some(weapon_entity) = weapon.find_target_entity(source_entity, world) {
            if world.get::<Weapon>(weapon_entity).is_some()
                && A::can_perform_with(weapon_entity, world)
            {
                // weapon exists and is the correct type of weapon
                return Ok(ChosenWeapon::Entity(weapon_entity));
            }
            let weapon_name =
                Description::get_reference_name(weapon_entity, Some(source_entity), world);
            return Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::Other(format!("You can't attack with {weapon_name}.")),
            });
        }
        return Err(InputParseError::CommandParseError {
            verb: verb_name.to_string(),
            error: CommandParseError::TargetNotFound(weapon),
        });
    }

    // no weapon provided
    Ok(ChosenWeapon::Unspecified)
}

/// Finds the weapon for the provided entity to use in an attack. Returns an `Err(ActionResult)` with an error message if no weapon can be found.
pub fn find_weapon<A: AttackType>(
    performing_entity: Entity,
    chosen_weapon: ChosenWeapon,
    world: &World,
) -> Result<Entity, ActionResult> {
    if let Some(e) = chosen_weapon.get_entity::<A>(performing_entity, world) {
        return Ok(e);
    }

    // weapon wasn't found
    Err(ActionResult::builder()
        .with_error(
            performing_entity,
            "You don't have a suitable weapon to make that attack with.".to_string(),
        )
        .build_complete_no_tick(false))
}

/// Makes the provided entities enter combat with each other, if they're not already in combat.
pub fn handle_begin_attack(
    attacker: Entity,
    target: Entity,
    result_builder: ActionResultBuilder,
    world: &mut World,
) -> (ActionResultBuilder, CombatRange) {
    let range = CombatState::get_entities_in_combat_with(attacker, world)
        .get(&target)
        .copied()
        .unwrap_or_else(|| determine_starting_range(attacker, target, world));

    (
        handle_enter_combat(attacker, target, range, result_builder, world),
        range,
    )
}

/// Determines the range the provided entities should begin combat at based on their weapons.
fn determine_starting_range(entity_1: Entity, entity_2: Entity, world: &World) -> CombatRange {
    let range_1 = Weapon::get_primary(entity_1, world)
        .map(|(weapon, _)| *weapon.ranges.usable.end())
        .unwrap_or(CombatRange::Longest);

    let range_2 = Weapon::get_primary(entity_2, world)
        .map(|(weapon, _)| *weapon.ranges.usable.end())
        .unwrap_or(CombatRange::Longest);

    range_1.max(range_2)
}

/// Makes the provided entities enter combat with each other at the provided range, if they're not already in combat.
pub fn handle_enter_combat(
    attacker: Entity,
    target: Entity,
    range: CombatRange,
    mut result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    if !CombatState::get_entities_in_combat_with(attacker, world)
        .keys()
        .contains(&target)
    {
        CombatState::set_in_combat(attacker, target, range, world);

        let message_format =
            MessageFormat::new("${attacker.Name} ${attacker.you:attack/attacks} ${target.name}!")
                .expect("message format should be valid");
        let message_tokens = BasicTokens::new()
            .with_entity("attacker".into(), attacker)
            .with_entity("target".into(), target);

        result_builder = result_builder
            .with_message(
                attacker,
                message_format
                    .interpolate(attacker, &message_tokens, world)
                    .expect("enter combat message interpolation shold not fail"),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(attacker),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    message_format,
                    message_tokens,
                ),
                world,
            );
    }

    result_builder
}

/// Builds an `ActionResult` with messages about how `entity` can't hit `target` with `weapon_entity`.
pub fn handle_hit_error(
    entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    error: HitError,
    result_builder: ActionResultBuilder,
    world: &World,
) -> ActionResult {
    match error {
        HitError::WeaponUnusable(weapon_unusable_error) => handle_weapon_unusable_error(
            entity,
            target,
            weapon_entity,
            weapon_unusable_error,
            result_builder,
            world,
        ),
        HitError::TargetHasNoBodyParts => result_builder
            .with_dynamic_message(
                Some(entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${entity.Name} can't seem to figure out how to hit ${target.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("entity".into(), entity)
                        .with_entity("target".into(), target),
                ),
                world,
            )
            .build_complete_should_tick(false),
    }
}

/// Builds an `ActionResult` with messages about how `entity` can't use `weapon_entity`.
pub fn handle_weapon_unusable_error(
    entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    error: WeaponUnusableError,
    result_builder: ActionResultBuilder,
    world: &World,
) -> ActionResult {
    let weapon_name = Description::get_reference_name(weapon_entity, Some(entity), world);
    let reason = match error {
        WeaponUnusableError::StatRequirementNotMet(requirement) => format!(
            "your {} is less than {:.1}",
            requirement.stat, requirement.min
        ),
        WeaponUnusableError::OutsideUsableRange { usable, actual } => {
            let distance_phrase = if actual < *usable.start() {
                "close to"
            } else {
                "far away from"
            };
            let target_name = Description::get_reference_name(target, Some(entity), world);
            format!("you are too {distance_phrase} {target_name}")
        }
    };

    result_builder
        .with_message(
            entity,
            format!("You can't use {weapon_name} because {reason}."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_dynamic_message(
            Some(entity),
            DynamicMessageLocation::SourceEntity,
            DynamicMessage::new_third_person(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                MessageFormat::new("${entity.Name} flails about uselessly with ${weapon.name}.")
                    .expect("message format should be valid"),
                BasicTokens::new().with_entity("entity".into(), entity),
            ),
            world,
        )
        .build_complete_should_tick(false)
}

/// Describes a hit.
pub struct HitParams {
    /// The entity doing the hitting
    pub performing_entity: Entity,
    /// The entity getting hit
    pub target: Entity,
    /// The weapon used
    pub weapon_entity: Entity,
    /// The damage done
    pub damage: u32,
    /// Whether the hit is a critical hit or not
    pub is_crit: bool,
    /// The body part hit on the entity getting hit
    pub body_part: Entity,
}

/// An error generated when checking for a hit during an attack
pub enum HitError {
    /// The weapon trying to be used in the attack is unusable
    WeaponUnusable(WeaponUnusableError),
    /// The target entity has no body parts to hit
    TargetHasNoBodyParts,
}

/// Performs a check to see if `attacker` hits `target` with `weapon`.
/// Returns `Some` if it was a hit, `Ok(None)` if it was a miss, and `Err` if the weapon is unusable or the target has no body parts.
pub fn check_for_hit(
    attacker: Entity,
    target: Entity,
    weapon_entity: Entity,
    range: CombatRange,
    to_hit_modification: f32,
    world: &mut World,
) -> Result<Option<HitParams>, HitError> {
    let weapon = world
        .get::<Weapon>(weapon_entity)
        .expect("weapon should be a weapon");
    let primary_weapon_stat = WeaponTypeStatCatalog::get_stats(&weapon.weapon_type, world).primary;

    let (to_hit_result, _) = Stats::check_vs(
        VsParticipant {
            entity: attacker,
            stat: primary_weapon_stat,
            modifiers: CheckModifiers::modify_value(to_hit_modification),
        },
        VsParticipant {
            entity: target,
            stat: Skill::Dodge.into(),
            modifiers: CheckModifiers::none(),
        },
        VsCheckParams::second_wins_ties(STANDARD_CHECK_XP),
        world,
    );

    // need to re-borrow this since `check_vs` takes a mutable `World`
    let weapon = world
        .get::<Weapon>(weapon_entity)
        .expect("weapon should be a weapon");

    if let Some(body_part_entity) = BodyPart::random_weighted(target, world) {
        if to_hit_result.succeeded() {
            let critical = to_hit_result == CheckResult::ExtremeSuccess;
            match weapon.calculate_damage(attacker, range, critical, world) {
                Ok(base_damage) => {
                    let damage_mult = world
                        .get::<BodyPartDamageMultiplier>(body_part_entity)
                        .map(|m| m.0)
                        .unwrap_or(1.0);
                    let damage = base_damage.mul_and_round(damage_mult);
                    Ok(Some(HitParams {
                        performing_entity: attacker,
                        target,
                        weapon_entity,
                        damage,
                        is_crit: critical,
                        body_part: body_part_entity,
                    }))
                }
                Err(e) => Err(HitError::WeaponUnusable(e)),
            }
        } else {
            // miss
            Ok(None)
        }
    } else {
        Err(HitError::TargetHasNoBodyParts)
    }
}

/// Does damage based on `hit_params` and adds messages to `result_builder` describing the hit.
pub fn handle_damage<A: AttackType>(
    hit_params: HitParams,
    result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    let weapon_messages = A::get_messages(hit_params.weapon_entity, world);

    let target_health = world
        .get::<Vitals>(hit_params.target)
        .map(|vitals| &vitals.health)
        .expect("target should have vitals");
    let damage_fraction = hit_params.damage as f32 / target_health.get_max();

    let hit_messages_to_choose_from = if damage_fraction >= HIGH_DAMAGE_THRESHOLD {
        weapon_messages.map(|m| &m.major_hit)
    } else if damage_fraction > LOW_DAMAGE_THRESHOLD {
        weapon_messages.map(|m: &crate::WeaponMessages| &m.regular_hit)
    } else {
        weapon_messages.map(|m| &m.minor_hit)
    };

    let hit_message = hit_messages_to_choose_from
        .and_then(|m| m.choose(&mut rand::thread_rng()).cloned())
        .unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.you:hit/hits} ${target.name's} ${body_part.plain_name} with ${weapon.name}.").expect("message format should be valid"));

    let hit_message_tokens = WeaponHitMessageTokens {
        attacker: hit_params.performing_entity,
        target: hit_params.target,
        weapon: hit_params.weapon_entity,
        body_part: hit_params.body_part,
    };

    result_builder.with_post_effect(Box::new(move |w| {
        VitalChange::<WeaponHitMessageTokens> {
            entity: hit_params.target,
            vital_type: VitalType::Health,
            operation: ValueChangeOperation::Subtract,
            amount: hit_params.damage as f32,
            message_params: vec![
                (
                    VitalChangeMessageParams::Dynamic(DynamicMessage::new(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                        hit_message,
                        hit_message_tokens,
                    )),
                    VitalChangeVisualizationType::Abbreviated,
                ),
                (
                    VitalChangeMessageParams::Direct {
                        entity: hit_params.target,
                        message: format!(
                            "Ow, {}!",
                            Description::get_reference_name(
                                hit_params.body_part,
                                Some(hit_params.target),
                                w
                            )
                        ),
                        category: MessageCategory::Internal(InternalMessageCategory::Misc),
                    },
                    VitalChangeVisualizationType::Full,
                ),
            ],
        }
        .apply(w);
    }))
}

/// Adds messages to `result_builder` describing a missed attack.
pub fn handle_miss<A: AttackType>(
    performing_entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    let miss_message = A::get_messages(weapon_entity, world)
        .map(|m| &m.miss)
        .and_then(|m| m.choose(&mut rand::thread_rng())).cloned()
        .unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.you:fail/fails} to hit ${target.name} with ${weapon.name}.").expect("message format should be valid"));

    let miss_message_tokens = WeaponMissMessageTokens {
        attacker: performing_entity,
        target,
        weapon: weapon_entity,
    };

    result_builder.with_dynamic_message(
        Some(performing_entity),
        DynamicMessageLocation::SourceEntity,
        DynamicMessage::new(
            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
            MessageDelay::Short,
            miss_message,
            miss_message_tokens,
        ),
        world,
    )
}

/// Verifies that everything is in order for an attack.
pub fn verify_combat_action_valid<A: AttackType>(
    notification: &Notification<VerifyActionNotification, A>,
    world: &World,
) -> VerifyResult {
    let verify_results = vec![
        verify_target_in_same_room(notification, world),
        verify_target_alive(notification, world),
        verify_attacker_wielding_weapon(notification, world),
    ];

    if verify_results.iter().all(|r| r.is_valid) {
        return VerifyResult::valid();
    }

    let messages = verify_results
        .into_iter()
        .flat_map(|r| r.messages)
        .collect::<HashMap<Entity, Vec<GameMessage>>>();
    VerifyResult::invalid_with_messages(messages)
}

/// Verifies that the target is in the same room as the attacker.
fn verify_target_in_same_room<A: AttackType>(
    notification: &Notification<VerifyActionNotification, A>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let target = notification.contents.get_target();
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    let attacker_location = world.get::<Location>(performing_entity);
    let target_location = world.get::<Location>(target);

    if attacker_location.is_none()
        || target_location.is_none()
        || attacker_location != target_location
    {
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("{target_name} is not here.")),
        );
    }

    VerifyResult::valid()
}

/// Verifies that the target is alive.
fn verify_target_alive<A: AttackType>(
    notification: &Notification<VerifyActionNotification, A>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let target = notification.contents.get_target();
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    if is_living_entity(target, world) {
        return VerifyResult::valid();
    }

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("{target_name} is not alive.")),
    )
}

/// Verifies that the attacker has the weapon they're trying to attack with.
fn verify_attacker_wielding_weapon<A: AttackType>(
    notification: &Notification<VerifyActionNotification, A>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    if let Some(weapon_entity) = notification
        .contents
        .get_weapon()
        .get_entity::<A>(performing_entity, world)
    {
        if EquippedItems::is_equipped(performing_entity, weapon_entity, world) {
            return VerifyResult::valid();
        }

        // if at least one hand is empty, treat it as being an innate weapon
        if let Some(equipped_items) = world.get::<EquippedItems>(performing_entity) {
            if equipped_items.get_num_hands_free(world) > 0 {
                if let Some((_, innate_weapon_entity)) = InnateWeapon::get(performing_entity, world)
                {
                    if weapon_entity == innate_weapon_entity {
                        return VerifyResult::valid();
                    }
                }
            }
        }

        let weapon_name =
            Description::get_reference_name(weapon_entity, Some(performing_entity), world);

        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("You don't have {weapon_name} equipped.")),
        );
    }

    VerifyResult::valid()
}

/// Queues an action to equip the weapon the attacker is trying to attack with, if they don't already have it equipped.
pub fn equip_before_attack<A: AttackType>(
    notification: &Notification<BeforeActionNotification, A>,
    world: &mut World,
) {
    let performing_entity = notification.notification_type.performing_entity;
    if let Some(weapon_entity) = notification
        .contents
        .get_weapon()
        .get_entity::<A>(performing_entity, world)
    {
        if EquippedItems::is_equipped(performing_entity, weapon_entity, world) {
            // the weapon is already equipped, no need to do anything
            return;
        }

        // if the weapon is an innate weapon, and the attacker has no free hands, unequip something
        if let Some((_, innate_weapon_entity)) = InnateWeapon::get(performing_entity, world) {
            if weapon_entity == innate_weapon_entity {
                let items_to_unequip =
                    EquippedItems::get_items_to_unequip_to_free_hands(performing_entity, 1, world);
                for item in items_to_unequip {
                    ActionQueue::queue_first(
                        world,
                        performing_entity,
                        Box::new(EquipAction {
                            target: item,
                            should_be_equipped: false,
                            notification_sender: ActionNotificationSender::new(),
                        }),
                    );
                }
                return;
            }
        }

        // the weapon isn't an innate weapon, and it's not equipped, so try to equip it
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(EquipAction {
                target: weapon_entity,
                should_be_equipped: true,
                notification_sender: ActionNotificationSender::new(),
            }),
        );
    }
}

/// Cancels any queued attacks when combat ends.
pub fn cancel_attacks_when_exit_combat(
    notification: &Notification<ExitCombatNotification, ()>,
    world: &mut World,
) {
    ActionQueue::cancel(
        is_combat_action,
        world,
        notification.notification_type.entity_1,
    );
    ActionQueue::cancel(
        is_combat_action,
        world,
        notification.notification_type.entity_2,
    );
}

fn is_combat_action(action: &dyn Action) -> bool {
    action.get_tags().contains(&ActionTag::Combat)
}
