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
    /// Any stat requirements for using the weapon.
    pub stat_requirements: Vec<WeaponStatRequirement>,
    /// Parameters for bonuses based on the weapon user's stats.
    pub stat_bonuses: WeaponStatBonuses,
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
    pub usable: RangeInclusive<CombatRange>,
    /// The ranges at which the weapon performs best.
    pub optimal: RangeInclusive<CombatRange>,
    /// The penalty to hit applied for each range level away from the optimal range.
    pub to_hit_penalty: u16,
    /// The penalty to damage applied for each range level away from the optimal range.
    pub damage_penalty: u32,
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

/// A stat requirement to use a weapon.
#[derive(Clone)]
pub struct WeaponStatRequirement {
    /// The required stat.
    pub stat: Stat,
    /// The minimum stat value to avoid penalties.
    pub min: f32,
    /// What to do if the stat requirement isn't met.
    pub below_min_behavior: WeaponStatRequirementNotMetBehavior,
}

/// What to do if a weapon stat requirement is not met.
#[derive(Clone)]
pub enum WeaponStatRequirementNotMetBehavior {
    /// The weapon cannot be used at all.
    Unusable,
    /// Adjustments are applied once if the stat requirement is not met, regardless of how much the stat requirement is not met by.
    FlatAdjustments(Vec<WeaponPerformanceAdjustment>),
    /// Adjustments are applied once for each point the using entity is below the stat requirement.
    AdjustmentsPerPointBelowMin(Vec<WeaponPerformanceAdjustment>),
}

/// Describes how to adjust the performance of a weapon when attacking with it.
#[derive(Clone)]
pub enum WeaponPerformanceAdjustment {
    /// Change the damage done by the weapon.
    Damage(WeaponDamageAdjustment),
    /// Change the likelihood of hitting with the weapon.
    ToHit(WeaponToHitAdjustment),
}

/// Describes how to adjust the damage of a weapon.
#[derive(Clone)]
pub enum WeaponDamageAdjustment {
    /// Roll for damage from a different range.
    NewRange(RangeInclusive<u32>),
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
}

/// Describes how to adjust the likelihood of hitting with a weapon.
#[derive(Clone)]
pub enum WeaponToHitAdjustment {
    /// Add some amount to the stat value for the to-hit roll.
    Add(i32),
    /// Multiply the stat value for the to-hit roll by some amount.
    Multiply(f32),
}

/// Describes bonuses to a weapon based on a user's stats.
pub struct WeaponStatBonuses {
    /// The values for which the damage bonus stat provides more damage bonus.
    /// * If the stat is less than the start of this range, no damage bonus will be applied.
    /// * If the stat is greater than the end of this range, the stat will be treated as if it were equal to the end of the range.
    pub damage_bonus_stat_range: RangeInclusive<f32>,
    /// The amount of extra damage done per point in the weapon's damage bonus stat above the start and up to the end of the damage bonus stat range.
    pub damage_bonus_per_stat_point: f32,
    /// The values for which the to-hit bonus stat provides more to-hit bonus.
    /// * If the stat is less than the start of this range, no to-hit bonus will be applied.
    /// * If the stat is greater than the end of this range, the stat will be treated as if it were equal to the end of the range.
    pub to_hit_bonus_stat_range: RangeInclusive<f32>,
    /// The to-hit bonus per point in the weapon's to-hit bonus stat above the start and up to the end of the to-hit bonus stat range.
    pub to_hit_bonus_per_stat_point: f32,
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
    pub fn get_effective_damage_range(
        &self,
        entity: Entity,
        world: &World,
    ) -> Result<RangeInclusive<u32>, WeaponUnusableError> {
        let min = *self.base_damage_range.start() as f32;
        let max = *self.base_damage_range.end() as f32;
        let stat_bonus = get_stat_bonus_damage(self, entity, world);
        let new_min = apply_stat_requirements_to_damage(min + stat_bonus, self, entity, world)?;
        let new_max = apply_stat_requirements_to_damage(max + stat_bonus, self, entity, world)?;

        Ok(new_min.round() as u32..=new_max.round() as u32)
    }

    /// Determines the to-hit bonus or penalty the provided entity has with this weapon based on their stats.
    pub fn get_effective_to_hit_modification(
        &self,
        entity: Entity,
        world: &World,
    ) -> Result<i16, WeaponUnusableError> {
        //TODO take into account stat requirements
        Ok(get_stat_to_hit_bonus(self, entity, world).round() as i16)
    }

    /// Calculates the total bonus or penalty for the provided entity to hit with this weapon at the provided range.
    pub fn calculate_to_hit_modification(
        &self,
        entity: Entity,
        range: CombatRange,
        world: &World,
    ) -> Result<i16, WeaponUnusableError> {
        if !self.ranges.usable.contains(&range) {
            return Err(WeaponUnusableError::OutsideUsableRange(
                self.ranges.usable.clone(),
            ));
        }

        let stat_modification = self.get_effective_to_hit_modification(entity, world)?;
        let range_penalty =
            self.ranges.to_hit_penalty * self.get_absolute_optimal_range_diff(range);

        Ok(stat_modification.saturating_sub_unsigned(range_penalty))
    }

    /// Calculates the amount of damage for a single hit from this weapon by the provided entity.
    pub fn calculate_damage(
        &self,
        attacking_entity: Entity,
        range: CombatRange,
        critical: bool,
        world: &World,
    ) -> Result<u32, WeaponUnusableError> {
        if !self.ranges.usable.contains(&range) {
            return Err(WeaponUnusableError::OutsideUsableRange(
                self.ranges.usable.clone(),
            ));
        }

        let mut base_damage_range = &self.get_effective_damage_range(attacking_entity, world)?;
        if critical {
            if let WeaponDamageAdjustment::NewRange(new_damage_range) =
                &self.critical_damage_behavior
            {
                base_damage_range = new_damage_range;
            }
        }

        let range_diff = self.get_absolute_optimal_range_diff(range);
        let damage_penalty = u32::from(range_diff) * self.ranges.damage_penalty;
        let mut min_damage = base_damage_range.start().saturating_sub(damage_penalty);
        let mut max_damage = base_damage_range.end().saturating_sub(damage_penalty);

        if critical {
            //TODO call `apply_damage_adjustment` instead
            match &self.critical_damage_behavior {
                WeaponDamageAdjustment::NewRange(_) => (), // already handled above
                WeaponDamageAdjustment::Set(damage) => return Ok(*damage),
                WeaponDamageAdjustment::Add(amount) => {
                    min_damage = min_damage.saturating_add_signed(*amount);
                    max_damage = max_damage.saturating_add_signed(*amount);
                }
                WeaponDamageAdjustment::Multiply(mult) => {
                    let new_min_damage = min_damage as f32 * mult;
                    let new_max_damage = max_damage as f32 * mult;
                    min_damage = new_min_damage.round() as u32;
                    max_damage = new_max_damage.round() as u32;
                }
                WeaponDamageAdjustment::Min => return Ok(min_damage),
                WeaponDamageAdjustment::Max => return Ok(max_damage),
            }
        }

        let mut rng = thread_rng();
        Ok(rng.gen_range(min_damage..=max_damage))
    }

    /// Determines how many range levels the provided range is outside of the optimal ranges, and in which direction.
    ///
    /// * If the provided range is shorter than the minimum optimal range, a negative number will be returned.
    /// * If the provided range is longer than the maximum optimal range, a positive number will be returned.
    /// * If the provided range is an optimal range, 0 will be returned.
    fn get_optimal_range_diff(&self, range: CombatRange) -> i16 {
        let range_number = range as u8;

        let min_range_number = *self.ranges.optimal.start() as u8;
        if range_number < min_range_number {
            return -i16::from(min_range_number - range_number);
        }

        let max_range_number = *self.ranges.optimal.end() as u8;
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

/// Gets the bonus to damage the provided entity does with the provided weapon based on their stats.
fn get_stat_bonus_damage(weapon: &Weapon, entity: Entity, world: &World) -> f32 {
    if let Some(stat) =
        WeaponTypeBonusStatCatalog::get_bonus_stats(&weapon.weapon_type, world).damage
    {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        let effective_stat_value =
            stat_value.min(*weapon.stat_bonuses.damage_bonus_stat_range.end());
        let amount_above_bonus_start =
            (effective_stat_value - weapon.stat_bonuses.damage_bonus_stat_range.start()).max(0.0);
        amount_above_bonus_start * DAMAGE_BONUS_PER_STAT_POINT
    } else {
        0.0
    }
}

/// Gets the to-hit bonus the provided entity has with the provided weapon based on their stats.
fn get_stat_to_hit_bonus(weapon: &Weapon, entity: Entity, world: &World) -> f32 {
    if let Some(stat) =
        WeaponTypeBonusStatCatalog::get_bonus_stats(&weapon.weapon_type, world).to_hit
    {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        let effective_stat_value =
            stat_value.min(*weapon.stat_bonuses.to_hit_bonus_stat_range.end());
        let amount_above_bonus_start =
            (effective_stat_value - weapon.stat_bonuses.to_hit_bonus_stat_range.start()).max(0.0);
        amount_above_bonus_start * TO_HIT_BONUS_PER_STAT_POINT
    } else {
        0.0
    }
}

/// An error for when a weapon is currently unusable.
pub enum WeaponUnusableError {
    /// A stat requirement is not met.
    StatRequirementNotMet(WeaponStatRequirement),
    /// The weapon is outside its usable range.
    OutsideUsableRange(RangeInclusive<CombatRange>),
}

/// Applies modifications to the provided damage based on the weapon's stat requirements and the entity's stats.
fn apply_stat_requirements_to_damage(
    damage: f32,
    weapon: &Weapon,
    entity: Entity,
    world: &World,
) -> Result<f32, WeaponUnusableError> {
    let mut new_damage = damage;
    for requirement in &weapon.stat_requirements {
        let stat_value = requirement
            .stat
            .get_entity_value(entity, world)
            .unwrap_or(0.0);
        if stat_value < requirement.min {
            let points_below_min = (requirement.min - stat_value).round() as u32;
            match &requirement.below_min_behavior {
                WeaponStatRequirementNotMetBehavior::Unusable => {
                    return Err(WeaponUnusableError::StatRequirementNotMet(
                        requirement.clone(),
                    ));
                }
                WeaponStatRequirementNotMetBehavior::FlatAdjustments(adjustments) => {
                    new_damage = apply_damage_adjustments(new_damage, adjustments, 1);
                }
                WeaponStatRequirementNotMetBehavior::AdjustmentsPerPointBelowMin(adjustments) => {
                    new_damage =
                        apply_damage_adjustments(new_damage, adjustments, points_below_min);
                }
            }
        }
    }

    Ok(new_damage)
}

fn apply_damage_adjustments(
    damage: f32,
    adjustments: &[WeaponPerformanceAdjustment],
    times: u32,
) -> f32 {
    let mut new_damage = damage;
    for adjustment in adjustments {
        if let WeaponPerformanceAdjustment::Damage(damage_adjustment) = adjustment {
            new_damage = apply_damage_adjustment(damage, damage_adjustment, times);
        }
    }

    new_damage
}

fn apply_damage_adjustment(damage: f32, adjustment: &WeaponDamageAdjustment, times: u32) -> f32 {
    match adjustment {
        WeaponDamageAdjustment::NewRange(_) => todo!(), //TODO this has to be done earlier
        WeaponDamageAdjustment::Set(x) => *x as f32,
        WeaponDamageAdjustment::Add(x) => damage + (*x as f32 * times as f32),
        WeaponDamageAdjustment::Multiply(x) => damage * x * times as f32,
        WeaponDamageAdjustment::Min => todo!(), //TODO this needs the range
        WeaponDamageAdjustment::Max => todo!(), //TODO this needs the range
    }
}

/// Applies modifications to the provided to-hit based on the weapon's stat requirements and the entity's stats.
fn apply_stat_requirements_to_to_hit(
    to_hit: f32,
    weapon: &Weapon,
    entity: Entity,
    world: &World,
) -> Result<f32, WeaponUnusableError> {
    let mut new_to_hit = to_hit;
    for requirement in &weapon.stat_requirements {
        let stat_value = requirement
            .stat
            .get_entity_value(entity, world)
            .unwrap_or(0.0);
        if stat_value < requirement.min {
            let points_below_min = (requirement.min - stat_value).round() as u32;
            match &requirement.below_min_behavior {
                WeaponStatRequirementNotMetBehavior::Unusable => {
                    return Err(WeaponUnusableError::StatRequirementNotMet(
                        requirement.clone(),
                    ));
                }
                WeaponStatRequirementNotMetBehavior::FlatAdjustments(adjustments) => {
                    new_to_hit = apply_to_hit_adjustments(new_to_hit, adjustments, 1);
                }
                WeaponStatRequirementNotMetBehavior::AdjustmentsPerPointBelowMin(adjustments) => {
                    new_to_hit =
                        apply_to_hit_adjustments(new_to_hit, adjustments, points_below_min);
                }
            }
        }
    }

    Ok(new_to_hit)
}

fn apply_to_hit_adjustments(
    to_hit: f32,
    adjustments: &[WeaponPerformanceAdjustment],
    times: u32,
) -> f32 {
    let mut new_to_hit = to_hit;
    for adjustment in adjustments {
        if let WeaponPerformanceAdjustment::ToHit(to_hit_adjustment) = adjustment {
            new_to_hit = apply_to_hit_adjustment(to_hit, to_hit_adjustment, times);
        }
    }

    new_to_hit
}

fn apply_to_hit_adjustment(to_hit: f32, adjustment: &WeaponToHitAdjustment, times: u32) -> f32 {
    match adjustment {
        WeaponToHitAdjustment::Add(x) => to_hit + (*x as f32 * times as f32),
        WeaponToHitAdjustment::Multiply(x) => to_hit * x * times as f32,
    }
}
