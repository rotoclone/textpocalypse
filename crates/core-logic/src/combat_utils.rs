use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    ActionResult, ActionResultBuilder, BodyPart, CombatRange, CombatState, Description,
    IntegerExtensions, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, ThirdPersonMessage, ThirdPersonMessageLocation, Weapon,
    WeaponUnusableError,
};

/// Multiplier applied to damage done to the head.
const HEAD_DAMAGE_MULT: f32 = 1.2;

/// Multiplier applied to damage done to the torso.
const TORSO_DAMAGE_MULT: f32 = 1.0;

/// Multiplier applied to damage done to non-head and non-torso body parts.
const APPENDAGE_DAMAGE_MULT: f32 = 0.8;

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
