use std::ops::RangeInclusive;

use bevy_ecs::prelude::*;
use rand::{thread_rng, Rng};
use strum::EnumIter;

use crate::{resource::WeaponTypeBonusStatCatalog, verb_forms::VerbForms};

use super::{InnateWeapon, Stat};

/// The amount of extra damage done per point in a weapon's damage bonus stat.
const DAMAGE_BONUS_PER_STAT_POINT: f32 = 0.5;

/// The to-hit bonus per point in a weapon's to-hit bonus stat.
const TO_HIT_BONUS_PER_STAT_POINT: f32 = 0.5;

/// An entity that can deal damage.
#[derive(Component)]
pub struct Weapon {
    /// The type of weapon this is.
    pub weapon_type: WeaponType,
    /// The verb to use when describing hits from this weapon. E.g. shoot, stab, etc.
    pub hit_verb: VerbForms,
    /// The amount of damage the weapon can do.
    pub base_damage_range: RangeInclusive<u32>,
    /// How to modify the damage on a critical hit.
    pub critical_damage_behavior: WeaponDamageAdjustment,
    /// Relevant ranges for the weapon.
    pub ranges: WeaponRanges,
    /// Stat requirements for using the weapon.
    pub stat_requirements: WeaponStatRequirements,
}

/// Represents a type of weapon.
#[derive(PartialEq, Eq, Hash, Clone, EnumIter)]
pub enum WeaponType {
    /// A shooty weapon
    Firearm,
    /// A bow
    Bow,
    /// A bladed weapon
    Blade,
    /// A smacky weapon
    Bludgeon,
    /// Just regular old fists
    Fists,
    /// A mod-defined weapon type
    Custom(String),
}

/// Describes the ranges at which a weapon can be used.
pub struct WeaponRanges {
    /// The ranges at which the weapon can be used at all.
    pub usable_ranges: RangeInclusive<CombatRange>,
    /// The ranges at which the weapon performs best.
    pub optimal_ranges: RangeInclusive<CombatRange>,
    /// The penalty to hit applied for each range level away from the optimal range.
    pub range_to_hit_penalty: u16,
    /// The penalty to damage applied for each range level away from the optimal range.
    pub range_damage_penalty: u32,
}

/// Represents how far away two combatants are from each other.
#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum CombatRange {
    Shortest,
    Short,
    Medium,
    Long,
    Longest,
}

pub struct WeaponStatRequirements {
    /// Any stat requirements for using the weapon.
    pub requirements: Vec<WeaponStatRequirement>,
    /// The value at which the damage bonus stat starts providing a damage bonus.
    pub damage_bonus_stat_start: f32,
    /// The amount of extra damage done per point in the weapon's damage bonus stat above `damage_bonus_stat_start`.
    pub damage_bonus_per_stat_point: f32,
    /// The value at which the to-hit bonus stat starts providing a to-hit bonus.
    pub to_hit_bonus_stat_start: f32,
    /// The to-hit bonus per point in the weapon's to-hit bonus stat above `to_hit_bonus_stat_start`.
    pub to_hit_bonus_per_stat_point: f32,
}

pub struct WeaponStatRequirement {
    pub stat: Stat,
    pub min: f32,
    pub below_min_behavior: WeaponStatRequirementNotMetBehavior,
}

/// What to do if a weapon stat requirement is not met.
pub enum WeaponStatRequirementNotMetBehavior {
    /// The weapon cannot be used at all.
    WeaponUnusable,
    /// Adjustments are applied regardless of how much the stat requirement is not met by.
    FlatAdjustments(Vec<WeaponPerformanceAdjustment>),
    /// Adjustments are applied for each point the using entity is below the stat requirement.
    AdjustmentsPerPointBelowMin(Vec<WeaponPerformanceAdjustment>),
}

/// Describes how to adjust the performance of a weapon when attacking with it.
pub enum WeaponPerformanceAdjustment {
    /// Change the damage done by the weapon.
    Damage(WeaponDamageAdjustment),
    /// Change the likelihood of hitting with the weapon.
    ToHit(WeaponToHitAdjustment),
}

/// Describes how to adjust the damage of a weapon.
pub enum WeaponDamageAdjustment {
    /// Instead of rolling for damage, do a specific amount of damage.
    Set(u32),
    /// Add some amount to the damage done.
    Add(i32),
    /// Multiply the damage done by some amount.
    Multiply(f32),
    /// Instead of rolling for damage, choose the lowest damage in the range.
    Min,
    /// Instead of rolling for damage, choose the highest damage in the range.
    Max,
    /// Roll for damage from a different range.
    NewRange(RangeInclusive<u32>),
}

/// Describes how to adjust the likelihood of hitting with a weapon.
pub enum WeaponToHitAdjustment {
    /// Add some amount to the stat value for the to-hit roll.
    Add(i32),
    /// Multiply the stat value for the to-hit roll by some amount.
    Multiply(f32),
}

impl Weapon {
    /// Gets the primary weapon the provided entity has equipped, along with its name.
    /// If the entity has no weapons equipped, its innate weapon will be returned.
    /// If the entity has no weapons equipped and no innate weapon, `None` will be returned.
    pub fn get_primary(entity: Entity, world: &World) -> Option<(&Weapon, &String)> {
        //TODO actually determine which equipped weapon is the primary one
        if let Some((weapon, weapon_name)) = Self::get_equipped(entity, world).first() {
            return Some((weapon, weapon_name));
        }

        world
            .get::<InnateWeapon>(entity)
            .map(|innate_weapon| (&innate_weapon.weapon, &innate_weapon.name))
    }

    /// Gets all the weapons the provided entity has equipped, along with their names.
    pub fn get_equipped(entity: Entity, world: &World) -> Vec<(&Weapon, &String)> {
        vec![] //TODO actually find equipped weapons
    }

    /// Determines the damage range the provided entity has with this weapon based on their stats.
    pub fn get_effective_damage_range(&self, entity: Entity, world: &World) -> RangeInclusive<u32> {
        let modification = get_damage_modification(&self, entity, world).round() as i32;
        let new_min = self
            .base_damage_range
            .start()
            .saturating_add_signed(modification);
        let new_max = self
            .base_damage_range
            .end()
            .saturating_add_signed(modification);

        new_min..=new_max
    }

    /// Determines the to-hit bonus or penalty the provided entity has with this weapon based on their stats.
    pub fn get_effective_to_hit_modification(&self, entity: Entity, world: &World) -> i16 {
        get_to_hit_modification(&self, entity, world).round() as i16
    }

    /// Calculates the total bonus or penalty for the provided entity to hit with this weapon at the provided range.
    pub fn calculate_to_hit_modification(
        &self,
        entity: Entity,
        range: CombatRange,
        world: &World,
    ) -> i16 {
        let stat_modification = self.get_effective_to_hit_modification(entity, world);
        let range_penalty = self.range_to_hit_penalty * self.get_absolute_optimal_range_diff(range);

        stat_modification.saturating_sub_unsigned(range_penalty)
    }

    /// Calculates the amount of damage for a single hit from this weapon by the provided entity.
    pub fn calculate_damage(
        &self,
        attacking_entity: Entity,
        range: CombatRange,
        critical: bool,
        world: &World,
    ) -> u32 {
        if !self.usable_ranges.contains(&range) {
            return 0;
        }

        let mut base_damage_range = &self.get_effective_damage_range(attacking_entity, world);
        if critical {
            if let CriticalDamageBehavior::NewRange(new_damage_range) =
                &self.critical_damage_behavior
            {
                base_damage_range = new_damage_range;
            }
        }

        let range_diff = self.get_absolute_optimal_range_diff(range);
        let damage_penalty = u32::from(range_diff) * self.range_damage_penalty;
        let mut min_damage = base_damage_range.start().saturating_sub(damage_penalty);
        let mut max_damage = base_damage_range.end().saturating_sub(damage_penalty);

        if critical {
            match &self.critical_damage_behavior {
                CriticalDamageBehavior::NewRange(_) => (), // already handled above
                CriticalDamageBehavior::Multiply(mult) => {
                    let new_min_damage = min_damage as f32 * mult;
                    let new_max_damage = max_damage as f32 * mult;
                    min_damage = new_min_damage.round() as u32;
                    max_damage = new_max_damage.round() as u32;
                }
                CriticalDamageBehavior::Amount(damage) => return *damage,
                CriticalDamageBehavior::Max => return max_damage,
            }
        }

        let mut rng = thread_rng();
        rng.gen_range(min_damage..=max_damage)
    }

    /// Determines how many range levels the provided range is outside of the optimal ranges, and in which direction.
    ///
    /// * If the provided range is shorter than the minimum optimal range, a negative number will be returned.
    /// * If the provided range is longer than the maximum optimal range, a positive number will be returned.
    /// * If the provided range is an optimal range, 0 will be returned.
    fn get_optimal_range_diff(&self, range: CombatRange) -> i16 {
        let range_number = range as u8;

        let min_range_number = *self.optimal_ranges.start() as u8;
        if range_number < min_range_number {
            return -i16::from(min_range_number - range_number);
        }

        let max_range_number = *self.optimal_ranges.end() as u8;
        if range_number > max_range_number {
            return i16::from(range_number - max_range_number);
        }

        0
    }

    /// Determines how many range levels the provided range is outside of the optimal ranges.
    fn get_absolute_optimal_range_diff(&self, range: CombatRange) -> u16 {
        self.get_optimal_range_diff(range).unsigned_abs()
    }
}

/// Gets the bonus or penalty to damage the provided entity does with the provided weapon based on their stats.
fn get_damage_modification(weapon: &Weapon, entity: Entity, world: &World) -> f32 {
    if let Some(stat) =
        WeaponTypeBonusStatCatalog::get_bonus_stats(&weapon.weapon_type, world).damage
    {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        stat_value * DAMAGE_BONUS_PER_STAT_POINT
    } else {
        0.0
    }
}

/// Gets the to-hit bonus or penalty the provided entity has with the provided weapon based on their stats.
fn get_to_hit_modification(weapon: &Weapon, entity: Entity, world: &World) -> f32 {
    if let Some(stat) =
        WeaponTypeBonusStatCatalog::get_bonus_stats(&weapon.weapon_type, world).to_hit
    {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        stat_value * TO_HIT_BONUS_PER_STAT_POINT
    } else {
        0.0
    }
}
