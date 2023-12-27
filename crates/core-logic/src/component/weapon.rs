use std::ops::RangeInclusive;

use bevy_ecs::prelude::*;
use rand::{thread_rng, Rng};

/// An entity that can deal damage.
#[derive(Component)]
pub struct Weapon {
    /// The type of weapon this is.
    pub weapon_type: WeaponType,
    /// The amount of damage the entity could do.
    pub base_damage_range: RangeInclusive<u32>,
    /// How to modify the damage on a critical hit.
    pub critical_damage_behavior: CriticalDamageBehavior,
    /// The ranges at which the weapon can be used at all.
    pub usable_ranges: RangeInclusive<CombatRange>,
    /// The ranges at which the weapon performs best.
    pub optimal_ranges: RangeInclusive<CombatRange>,
    /// The penalty to hit applied for each range level away from the optimal range.
    pub range_to_hit_penalty: u16,
    /// The penalty to damage applied for each range level away from the optimal range.
    pub range_damage_penalty: u32,
}

/// Represents a type of weapon.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
}

/// Describes how to modify damage done on a critical hit.
pub enum CriticalDamageBehavior {
    /// Roll for damage from a different range.
    NewRange(RangeInclusive<u32>),
    /// Multiply the damage done by some amount.
    Multiply(f32),
    /// Instead of rolling for damage, do a specific amount of damage.
    Amount(u32),
    /// Instead of rolling for damage, choose the highest damage in the range.
    Max,
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

impl Weapon {
    /// Calculates the penalty to hit with this weapon based on its range.
    pub fn calculate_to_hit_penalty(&self, range: CombatRange) -> u16 {
        self.range_to_hit_penalty * self.get_absolute_optimal_range_diff(range)
    }

    /// Calculates the amount of damage for a single hit from this weapon.
    pub fn calculate_damage(&self, range: CombatRange, critical: bool) -> u32 {
        if !self.usable_ranges.contains(&range) {
            return 0;
        }

        let mut base_damage_range = &self.base_damage_range;
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
