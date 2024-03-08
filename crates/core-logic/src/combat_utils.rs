use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    vital_change::ValueChangeOperation, ActionResult, ActionResultBuilder, BodyPart, CombatRange,
    CombatState, Description, IntegerExtensions, InternalMessageCategory, MessageCategory,
    MessageDelay, SurroundingsMessageCategory, ThirdPersonMessage, ThirdPersonMessageLocation,
    VitalChange, VitalType, Vitals, Weapon, WeaponUnusableError,
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

/// Makes the provided entities enter combat with each other, if they're not already in combat.
pub fn handle_begin_attack(
    attacker: Entity,
    target: Entity,
    mut result_builder: ActionResultBuilder,
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

        let target_name = Description::get_reference_name(target, Some(attacker), world);
        result_builder = result_builder
            .with_message(
                attacker,
                format!("You attack {target_name}!"),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(attacker),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                )
                .add_name(attacker)
                .add_string(" attacks ")
                .add_name(target)
                .add_string("!"),
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
    /// The body part hit
    pub body_part: BodyPart,
}

/// Does damage based on `hit_params` and adds messages to `result_builder` describing the hit.
pub fn handle_damage(
    hit_params: HitParams,
    mut result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    //TODO instead of having these messages in here, make them defined on the weapons themselves
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
    let weapon_hit_verb = &world
        .get::<Weapon>(hit_params.weapon_entity)
        .expect("weapon should be a weapon")
        .hit_verb;
    result_builder
        .with_message(
            hit_params.performing_entity,
            format!(
                "You {} {}'s {} with a {} from {}.",
                hit_severity_first_person,
                target_name,
                hit_params.body_part,
                weapon_hit_verb.second_person,
                weapon_name
            ),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(hit_params.performing_entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(hit_params.performing_entity)
            .add_string(format!(" {hit_severity_third_person} "))
            .add_name(hit_params.target)
            .add_string(format!(
                " in the {} with a {} from ",
                hit_params.body_part, weapon_hit_verb.second_person
            ))
            .add_name(hit_params.weapon_entity)
            .add_string("!"),
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
    result_builder
        .with_message(
            performing_entity,
            format!("You fail to hit {target_name} with {weapon_name}."),
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
            .add_name(performing_entity)
            .add_string(" fails to hit ")
            .add_name(target)
            .add_string(" with ")
            .add_name(weapon_entity)
            .add_string("."),
            world,
        )
}
