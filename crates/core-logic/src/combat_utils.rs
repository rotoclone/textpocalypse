use bevy_ecs::prelude::*;
use itertools::Itertools;
use rand::seq::SliceRandom;
use regex::{Captures, Regex};

use crate::{
    resource::WeaponTypeStatCatalog, vital_change::ValueChangeOperation, ActionResult,
    ActionResultBuilder, BasicTokens, BodyPart, CheckModifiers, CheckResult, CombatRange,
    CombatState, CommandParseError, CommandTarget, Container, Description, EquippedItems,
    InnateWeapon, InputParseError, IntegerExtensions, InternalMessageCategory, MessageCategory,
    MessageDelay, MessageFormat, Skill, Stats, SurroundingsMessageCategory, ThirdPersonMessage,
    ThirdPersonMessageLocation, VitalChange, VitalType, Vitals, VsCheckParams, VsParticipant,
    Weapon, WeaponHitMessageTokens, WeaponMissMessageTokens, WeaponUnusableError,
};

/// Multiplier applied to damage done to the head.
const HEAD_DAMAGE_MULT: f32 = 1.2;

/// Multiplier applied to damage done to the torso.
const TORSO_DAMAGE_MULT: f32 = 1.0;

/// Multiplier applied to damage done to non-head and non-torso body parts.
const APPENDAGE_DAMAGE_MULT: f32 = 0.8;

/// The fraction of a target's health that counts as a high amount of damage.
const HIGH_DAMAGE_THRESHOLD: f32 = 0.4;

/// The fraction of a target's health that counts as a low amount of damage.
const LOW_DAMAGE_THRESHOLD: f32 = 0.1;

/// Describes an attack, parsed into entities.
pub struct ParsedAttack {
    /// The target of the attack.
    pub target: Entity,
    /// The weapon to use for the attack.
    pub weapon: Entity,
}

/// Parses input from `source_entity` as an attack command.
/// `pattern` should have a capture group with the name provided in `target_capture_name`. Any other capture groups will be ignored.
/// `weapon_matcher` should return `true` when passed an entity that would be a valid weapon for use in the attack.
///
/// Returns `Ok` with the target entity, or `Err` if the input is invalid.
pub fn parse_attack_input<F>(
    input: &str,
    source_entity: Entity,
    pattern: &Regex,
    pattern_with_weapon: &Regex,
    target_capture_name: &str,
    weapon_capture_name: &str,
    verb_name: &str,
    weapon_matcher: F,
    world: &World,
) -> Result<ParsedAttack, InputParseError>
where
    F: Fn(Entity, &World) -> bool,
{
    if let Some(captures) = pattern_with_weapon.captures(input) {
        return parse_attack_input_captures(
            &captures,
            source_entity,
            target_capture_name,
            weapon_capture_name,
            verb_name,
            weapon_matcher,
            world,
        );
    }

    if let Some(captures) = pattern.captures(input) {
        return parse_attack_input_captures(
            &captures,
            source_entity,
            target_capture_name,
            weapon_capture_name,
            verb_name,
            weapon_matcher,
            world,
        );
    }

    Err(InputParseError::UnknownCommand)
}

fn parse_attack_input_captures<F>(
    captures: &Captures,
    source_entity: Entity,
    target_capture_name: &str,
    weapon_capture_name: &str,
    verb_name: &str,
    weapon_matcher: F,
    world: &World,
) -> Result<ParsedAttack, InputParseError>
where
    F: Fn(Entity, &World) -> bool,
{
    let target_entity = parse_attack_target(
        captures,
        target_capture_name,
        source_entity,
        verb_name,
        world,
    )?;
    let weapon_entity = parse_attack_weapon(
        captures,
        weapon_capture_name,
        source_entity,
        verb_name,
        weapon_matcher,
        world,
    )?;

    Ok(ParsedAttack {
        target: target_entity,
        weapon: weapon_entity,
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
/// Weapons valid for use in the attack will return `true` when passed into the provided `weapon_matcher`.
fn parse_attack_weapon<F>(
    captures: &Captures,
    weapon_capture_name: &str,
    source_entity: Entity,
    verb_name: &str,
    weapon_matcher: F,
    world: &World,
) -> Result<Entity, InputParseError>
where
    F: Fn(Entity, &World) -> bool,
{
    if let Some(target_match) = captures.name(weapon_capture_name) {
        let weapon = CommandTarget::parse(target_match.as_str());
        if let Some(weapon_entity) = weapon.find_target_entity(source_entity, world) {
            if world.get::<Weapon>(weapon_entity).is_some() && weapon_matcher(weapon_entity, world)
            {
                // weapon exists and is the correct type of weapon
                return Ok(weapon_entity);
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
    // prioritize the primary weapon
    if let Some((_, weapon_entity)) = Weapon::get_primary(source_entity, world) {
        if weapon_matcher(weapon_entity, world) {
            return Ok(weapon_entity);
        }
    }

    // primary weapon didn't match, so fall back to other equipped weapons
    if let Some(equipped_items) = world.get::<EquippedItems>(source_entity) {
        for item in equipped_items.get_items() {
            if world.get::<Weapon>(*item).is_some() && weapon_matcher(*item, world) {
                return Ok(*item);
            }
        }
    }

    // no equipped weapons matched, try innate weapon
    if let Some((_, innate_weapon_entity)) = InnateWeapon::get(source_entity, world) {
        if weapon_matcher(innate_weapon_entity, world) {
            return Ok(innate_weapon_entity);
        }
    }

    // no equipped weapons or innate weapon matched, fall back to non-equipped weapons
    if let Some(container) = world.get::<Container>(source_entity) {
        for item in container.get_entities(source_entity, world) {
            if world.get::<Weapon>(item).is_some() && weapon_matcher(item, world) {
                return Ok(item);
            }
        }
    }

    // couldn't find a matching weapon
    Err(InputParseError::CommandParseError {
        verb: verb_name.to_string(),
        error: CommandParseError::MissingTarget,
    })
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
            MessageFormat::new("${attacker.Name} ${attacker.attack/attacks} ${target.name}!")
                .expect("message format should be valid");
        let message_tokens = BasicTokens::new()
            .with_entity("attacker".into(), attacker)
            .with_entity("target".into(), target);

        let target_name = Description::get_reference_name(target, Some(attacker), world);
        result_builder = result_builder
            .with_message(
                attacker,
                message_format
                    .interpolate(attacker, &message_tokens, world)
                    .expect("enter combat message interpolation shold not fail"),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(attacker),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
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

/// Builds an `ActionResult` with messages about how `entity` can't use a weapon.
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
        .with_third_person_message(
            Some(entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(entity)
            .add_string(" flails about uselessly with ")
            .add_name(weapon_entity)
            .add_string("."),
            world,
        )
        .build_complete_should_tick(false)
}

/// Applies a multiplier to the provided damage based on the provided body part.
pub fn apply_body_part_damage_multiplier(base_damage: u32, body_part: BodyPart) -> u32 {
    let mult = match body_part {
        BodyPart::Head => HEAD_DAMAGE_MULT,
        BodyPart::Torso => TORSO_DAMAGE_MULT,
        _ => APPENDAGE_DAMAGE_MULT,
    };

    base_damage.mul_and_round(mult)
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
    /// The body part hit
    pub body_part: BodyPart,
}

/// Performs a check to see if `attacker` hits `target` with `weapon`.
/// Returns `Some` if it was a hit, `Ok(None)` if it was a miss, and `Err` if the weapon is unusable.
pub fn check_for_hit(
    attacker: Entity,
    target: Entity,
    weapon_entity: Entity,
    range: CombatRange,
    to_hit_modification: f32,
    world: &mut World,
) -> Result<Option<HitParams>, WeaponUnusableError> {
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
        VsCheckParams::second_wins_ties(),
        world,
    );

    // need to re-borrow this since `check_vs` takes a mutable `World`
    let weapon = world
        .get::<Weapon>(weapon_entity)
        .expect("weapon should be a weapon");

    let body_part = BodyPart::random_weighted(world);
    if to_hit_result.succeeded() {
        let critical = to_hit_result == CheckResult::ExtremeSuccess;
        match weapon.calculate_damage(attacker, range, critical, world) {
            Ok(x) => {
                let damage = apply_body_part_damage_multiplier(x, body_part);
                Ok(Some(HitParams {
                    performing_entity: attacker,
                    target,
                    weapon_entity,
                    damage,
                    is_crit: critical,
                    body_part,
                }))
            }
            Err(e) => Err(e),
        }
    } else {
        // miss
        Ok(None)
    }
}

/// Does damage based on `hit_params` and adds messages to `result_builder` describing the hit.
pub fn handle_damage(
    hit_params: HitParams,
    mut result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    //TODO replace is_crit with a hit severity enum based on percentage of target health removed?
    let target_health = world
        .get::<Vitals>(hit_params.target)
        .map(|vitals| &vitals.health)
        .expect("target should have vitals");
    let damage_fraction = hit_params.damage as f32 / target_health.get_max();
    let (hit_severity_first_person, hit_severity_third_person) =
        if damage_fraction >= HIGH_DAMAGE_THRESHOLD {
            ("mutilate", "mutilates")
        } else if damage_fraction > LOW_DAMAGE_THRESHOLD {
            ("hit", "hits")
        } else {
            ("barely scratch", "barely scratches")
        };

    result_builder = result_builder.with_post_effect(Box::new(move |w| {
        VitalChange {
            entity: hit_params.target,
            vital_type: VitalType::Health,
            operation: ValueChangeOperation::Subtract,
            amount: hit_params.damage as f32,
            message: Some(format!("Ow, your {}!", hit_params.body_part)),
        }
        .apply(w);
    }));

    let weapon_name = Description::get_reference_name(
        hit_params.weapon_entity,
        Some(hit_params.performing_entity),
        world,
    );
    let target_name = Description::get_reference_name(
        hit_params.target,
        Some(hit_params.performing_entity),
        world,
    );

    let weapon_messages = world
        .get::<Weapon>(hit_params.weapon_entity)
        .expect("weapon should be a weapon")
        .messages;

    let hit_messages_to_choose_from = if hit_params.is_crit {
        weapon_messages.crit
    } else {
        weapon_messages.hit
    };

    let hit_message = hit_messages_to_choose_from
        .choose(&mut rand::thread_rng())
        .cloned()
        .unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.hit/hits} ${target.name}'s ${body_part} with ${weapon.name}.").expect("message format should be valid"));

    let hit_message_tokens = WeaponHitMessageTokens {
        attacker: hit_params.performing_entity,
        target: hit_params.target,
        weapon: hit_params.weapon_entity,
        body_part: hit_params.body_part.to_string(),
    };

    result_builder
        .with_message(
            hit_params.performing_entity,
            hit_message
                .interpolate(hit_params.performing_entity, &hit_message_tokens, world)
                .expect("hit message interpolation should not fail"),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(hit_params.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                hit_message,
                hit_message_tokens,
            ),
            world,
        )
}

/// Adds messages to `result_builder` describing a missed attack.
pub fn handle_miss(
    performing_entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    let weapon_name =
        Description::get_reference_name(weapon_entity, Some(performing_entity), world);
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    let miss_message = world.get::<Weapon>(weapon_entity).expect("weapon should be a weapon").messages.miss.choose(&mut rand::thread_rng()).cloned().unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.fail/fails} to hit ${target.name} with ${weapon.name}.").expect("message format should be valid"));

    let miss_message_tokens = WeaponMissMessageTokens {
        attacker: performing_entity,
        target,
        weapon: weapon_entity,
    };

    result_builder
        .with_message(
            performing_entity,
            miss_message
                .interpolate(performing_entity, &miss_message_tokens, world)
                .expect("miss message should interpolate correctly"),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                miss_message,
                miss_message_tokens,
            ),
            world,
        )
}
