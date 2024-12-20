use std::ops::RangeInclusive;

use bevy_ecs::prelude::*;
use rand::{thread_rng, Rng};
use strum::EnumIter;

use crate::{
    component::EquippedItems,
    format_list,
    range_extensions::RangeExtensions,
    resource::{get_stat_name, WeaponTypeNameCatalog, WeaponTypeStatCatalog},
    AttributeSection, AttributeSectionName, ChosenWeapon, MessageFormat, MessageTokens,
    SectionAttributeDescription, TokenName, TokenValue,
};

use super::{
    combat_state::CombatRange, AttributeDescriber, AttributeDescription, AttributeDetailLevel,
    DescribeAttributes, InnateWeapon, Stat,
};

/// An entity that can deal damage.
#[derive(Component)]
pub struct Weapon {
    /// The type of weapon this is.
    pub weapon_type: WeaponType,
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
    /// The messages for default attacks with this weapon.
    pub default_attack_messages: WeaponMessages,
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

/// Trait for structs describing a type of attack.
pub trait AttackType: std::fmt::Debug {
    /// Determines whether the provided weapon entity can perform this attack.
    fn can_perform_with(weapon_entity: Entity, world: &World) -> bool;

    /// Gets the messages to use for attacks of this type with the provided weapon, if there are any defined.
    fn get_messages(weapon_entity: Entity, world: &World) -> Option<&WeaponMessages>;

    /// Gets the target of the attack.
    fn get_target(&self) -> Entity;

    /// Gets the weapon used in the attack.
    fn get_weapon(&self) -> ChosenWeapon;
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

/// Describes a weapon.
#[derive(Debug)]
struct WeaponAttributeDescriber;

impl AttributeDescriber for WeaponAttributeDescriber {
    fn describe(
        &self,
        pov_entity: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(weapon) = world.get::<Weapon>(entity) {
            let weapon_type_stats = WeaponTypeStatCatalog::get_stats(&weapon.weapon_type, world);

            let mut attributes = vec![
                SectionAttributeDescription {
                    name: "Type".to_string(),
                    description: WeaponTypeNameCatalog::get_name(&weapon.weapon_type, world),
                },
                SectionAttributeDescription {
                    name: "Primary stat".to_string(),
                    description: get_stat_name(&weapon_type_stats.primary, world),
                },
            ];

            if let Some(damage_bonus_stat) = weapon_type_stats.damage_bonus {
                attributes.push(SectionAttributeDescription {
                    name: "Damage bonus stat".to_string(),
                    description: get_stat_name(&damage_bonus_stat, world),
                });
            }

            if let Some(to_hit_bonus_stat) = weapon_type_stats.to_hit_bonus {
                attributes.push(SectionAttributeDescription {
                    name: "Accuracy bonus stat".to_string(),
                    description: get_stat_name(&to_hit_bonus_stat, world),
                });
            }

            if !weapon.stat_requirements.is_empty() {
                let requirement_descriptions = weapon
                    .stat_requirements
                    .iter()
                    .map(|req| {
                        let below_min_behavior = match &req.below_min_behavior {
                            WeaponStatRequirementNotMetBehavior::Unusable => "unusable".to_string(),
                            WeaponStatRequirementNotMetBehavior::FlatAdjustments(adjustments) => {
                                describe_weapon_performance_reduction(adjustments)
                            }
                            WeaponStatRequirementNotMetBehavior::AdjustmentsPerPointBelowMin(
                                adjustments,
                            ) => describe_weapon_performance_reduction(adjustments),
                        };

                        format!(
                            "{} below {:.1} {}",
                            below_min_behavior,
                            req.min,
                            get_stat_name(&req.stat, world)
                        )
                    })
                    .collect::<Vec<String>>();

                attributes.push(SectionAttributeDescription {
                    name: "Stat requirements".to_string(),
                    description: format_list(&requirement_descriptions),
                });
            }

            let base_damage_description = format!(
                "{}-{}",
                weapon.base_damage_range.start(),
                weapon.base_damage_range.end()
            );

            let effective_damage_description =
                match weapon.get_effective_damage_range(pov_entity, world) {
                    Ok(range) => format!("{}-{}", range.start(), range.end()),
                    Err(_) => "[unusable]".to_string(),
                };

            attributes.extend_from_slice(&[
                SectionAttributeDescription {
                    name: "Base damage".to_string(),
                    description: base_damage_description,
                },
                SectionAttributeDescription {
                    name: "Effective damage".to_string(),
                    description: effective_damage_description,
                },
                SectionAttributeDescription {
                    name: "Usable range".to_string(),
                    description: describe_range(&weapon.ranges.usable),
                },
                SectionAttributeDescription {
                    name: "Optimal range".to_string(),
                    description: describe_range(&weapon.ranges.optimal),
                },
            ]);

            return vec![AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Weapon,
                attributes,
            })];
        }
        Vec::new()
    }
}

fn describe_weapon_performance_reduction(adjustments: &[WeaponPerformanceAdjustment]) -> String {
    let damage_adjustment = adjustments
        .iter()
        .any(|adj| matches!(adj, WeaponPerformanceAdjustment::Damage(_)));
    let to_hit_adjustment = adjustments
        .iter()
        .any(|adj| matches!(adj, WeaponPerformanceAdjustment::ToHit(_)));

    if damage_adjustment && to_hit_adjustment {
        "reduced damage and accuracy".to_string()
    } else if damage_adjustment {
        "reduced damage".to_string()
    } else if to_hit_adjustment {
        "reduced accuracy".to_string()
    } else {
        "no penalty".to_string()
    }
}

fn describe_range(range: &RangeInclusive<CombatRange>) -> String {
    if range.start() == range.end() {
        range.start().to_string()
    } else {
        format!("{}-{}", range.start(), range.end())
    }
}

impl DescribeAttributes for Weapon {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(WeaponAttributeDescriber)
    }
}

/// Describes the messages to send when a weapon is used.
pub struct WeaponMessages {
    /// Messages for misses
    pub miss: Vec<MessageFormat<WeaponMissMessageTokens>>,
    /// Messages for hits that don't do much damage
    pub minor_hit: Vec<MessageFormat<WeaponHitMessageTokens>>,
    /// Messages for hits that do a normal amount of damage
    pub regular_hit: Vec<MessageFormat<WeaponHitMessageTokens>>,
    /// Messages for hits that do a lot of damage
    pub major_hit: Vec<MessageFormat<WeaponHitMessageTokens>>,
}

/// Tokens used in weapon messages for hits.
#[derive(Clone)]
pub struct WeaponHitMessageTokens {
    /// The attacking entity
    pub attacker: Entity,
    /// The target of the attack
    pub target: Entity,
    /// THe weapon used in the attack
    pub weapon: Entity,
    /// The body part hit in the attack
    pub body_part: Entity,
}

/// Tokens used in weapon messages for misses.
#[derive(Clone)]
pub struct WeaponMissMessageTokens {
    /// The attacking entity
    pub attacker: Entity,
    /// The target of the attack
    pub target: Entity,
    /// The weapon used in the attack
    pub weapon: Entity,
}

impl MessageTokens for WeaponHitMessageTokens {
    fn get_token_map(&self) -> std::collections::HashMap<TokenName, TokenValue> {
        [
            ("attacker".into(), TokenValue::Entity(self.attacker)),
            ("target".into(), TokenValue::Entity(self.target)),
            ("weapon".into(), TokenValue::Entity(self.weapon)),
            ("body_part".into(), TokenValue::Entity(self.body_part)),
        ]
        .into()
    }
}

impl MessageTokens for WeaponMissMessageTokens {
    fn get_token_map(&self) -> std::collections::HashMap<TokenName, TokenValue> {
        [
            ("attacker".into(), TokenValue::Entity(self.attacker)),
            ("target".into(), TokenValue::Entity(self.target)),
            ("weapon".into(), TokenValue::Entity(self.weapon)),
        ]
        .into()
    }
}

impl Weapon {
    /// Gets the primary weapon the provided entity has equipped, including the Entity of the weapon itself.
    /// * If the entity has no weapons equipped, its innate weapon will be returned.
    /// * If the entity has no weapons equipped and no innate weapon, `None` will be returned.
    pub fn get_primary(entity: Entity, world: &World) -> Option<(&Weapon, Entity)> {
        // assume the first-equipped weapon is the primary one
        if let Some((weapon, weapon_name)) = Self::get_equipped(entity, world).into_iter().next() {
            return Some((weapon, weapon_name));
        }

        InnateWeapon::get(entity, world)
    }

    /// Gets all the weapons the provided entity has equipped, along with their entities.
    /// Weapons will be returned in the order they were equipped, oldest first.
    pub fn get_equipped(entity: Entity, world: &World) -> Vec<(&Weapon, Entity)> {
        if let Some(equipped_items) = world.get::<EquippedItems>(entity) {
            equipped_items
                .get_items()
                .iter()
                .filter_map(|item| world.get::<Weapon>(*item).map(|weapon| (weapon, *item)))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Determines the damage range the provided entity has with this weapon based on their stats.
    pub fn get_effective_damage_range(
        &self,
        entity: Entity,
        world: &World,
    ) -> Result<RangeInclusive<u32>, WeaponUnusableError> {
        let stat_bonus = get_stat_bonus_damage(self, entity, world);

        let mut damage_range =
            RangeInclusive::<f32>::from_u32_range(self.base_damage_range.clone());
        damage_range = damage_range.add(stat_bonus);
        damage_range = apply_stat_requirements_to_damage_range(damage_range, self, entity, world)?;

        Ok(damage_range.as_u32_saturating())
    }

    /// Determines the to-hit bonus or penalty the provided entity has with this weapon in general based on their stats.
    pub fn get_effective_to_hit_modification(
        &self,
        entity: Entity,
        world: &World,
    ) -> Result<i16, WeaponUnusableError> {
        let stat = WeaponTypeStatCatalog::get_stats(&self.weapon_type, world).primary;
        let base_to_hit = stat.get_entity_value(entity, world).unwrap_or(0.0);
        let stat_bonus = get_stat_to_hit_bonus(self, entity, world);
        let mut modified_to_hit = base_to_hit + stat_bonus;
        modified_to_hit = apply_stat_requirements_to_to_hit(modified_to_hit, self, entity, world)?;

        Ok((modified_to_hit - base_to_hit)
            .round()
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16)
    }

    /// Calculates the total bonus or penalty for the provided entity to hit with this weapon at the provided range.
    pub fn calculate_to_hit_modification(
        &self,
        entity: Entity,
        range: CombatRange,
        world: &World,
    ) -> Result<i16, WeaponUnusableError> {
        if !self.ranges.usable.contains(&range) {
            return Err(WeaponUnusableError::OutsideUsableRange {
                usable: self.ranges.usable.clone(),
                actual: range,
            });
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
            return Err(WeaponUnusableError::OutsideUsableRange {
                usable: self.ranges.usable.clone(),
                actual: range,
            });
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
        let mut damage_range = RangeInclusive::<f32>::from_u32_range(base_damage_range.clone());
        damage_range = damage_range.sub(damage_penalty as f32);

        if critical {
            match &self.critical_damage_behavior {
                WeaponDamageAdjustment::NewRange(_) => (), // already handled above
                WeaponDamageAdjustment::Set(x) => return Ok(*x),
                WeaponDamageAdjustment::Add(x) => {
                    damage_range = damage_range.add(*x as f32);
                }
                WeaponDamageAdjustment::Multiply(x) => {
                    damage_range = damage_range.mult(*x);
                }
                WeaponDamageAdjustment::Min => {
                    return Ok(*damage_range.as_u32_saturating().start())
                }
                WeaponDamageAdjustment::Max => return Ok(*damage_range.as_u32_saturating().end()),
            }
        }

        let mut rng = thread_rng();
        Ok(rng.gen_range(damage_range.as_u32_saturating()))
    }

    /// Determines how many range levels the provided range is outside of the optimal ranges, and in which direction.
    ///
    /// * If the provided range is shorter than the minimum optimal range, a negative number will be returned.
    /// * If the provided range is longer than the maximum optimal range, a positive number will be returned.
    /// * If the provided range is an optimal range, 0 will be returned.
    pub fn get_optimal_range_diff(&self, range: CombatRange) -> i16 {
        Self::get_range_diff(range, &self.ranges.optimal)
    }

    /// Determines how many range levels the provided range is outside of the usable ranges, and in which direction.
    ///
    /// * If the provided range is shorter than the minimum usable range, a negative number will be returned.
    /// * If the provided range is longer than the maximum usable range, a positive number will be returned.
    /// * If the provided range is a usable range, 0 will be returned.
    pub fn get_usable_range_diff(&self, range: CombatRange) -> i16 {
        Self::get_range_diff(range, &self.ranges.usable)
    }

    /// Determines how many range levels the provided range is outside of the provided window, and in which direction.
    ///
    /// * If the provided range is shorter than the start of the window, a negative number will be returned.
    /// * If the provided range is longer than the end of the window, a positive number will be returned.
    /// * If the provided range is within the window, 0 will be returned.
    fn get_range_diff(range_to_check: CombatRange, window: &RangeInclusive<CombatRange>) -> i16 {
        let range_number = range_to_check as u8;

        let min_range_number = *window.start() as u8;
        if range_number < min_range_number {
            return -i16::from(min_range_number - range_number);
        }

        let max_range_number = *window.end() as u8;
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
    if let Some(stat) = WeaponTypeStatCatalog::get_stats(&weapon.weapon_type, world).damage_bonus {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        let effective_stat_value =
            stat_value.min(*weapon.stat_bonuses.damage_bonus_stat_range.end());
        let amount_above_bonus_start =
            (effective_stat_value - weapon.stat_bonuses.damage_bonus_stat_range.start()).max(0.0);
        amount_above_bonus_start * weapon.stat_bonuses.damage_bonus_per_stat_point
    } else {
        0.0
    }
}

/// Gets the to-hit bonus the provided entity has with the provided weapon based on their stats.
fn get_stat_to_hit_bonus(weapon: &Weapon, entity: Entity, world: &World) -> f32 {
    if let Some(stat) = WeaponTypeStatCatalog::get_stats(&weapon.weapon_type, world).to_hit_bonus {
        let stat_value = stat.get_entity_value(entity, world).unwrap_or(0.0);
        let effective_stat_value =
            stat_value.min(*weapon.stat_bonuses.to_hit_bonus_stat_range.end());
        let amount_above_bonus_start =
            (effective_stat_value - weapon.stat_bonuses.to_hit_bonus_stat_range.start()).max(0.0);
        amount_above_bonus_start * weapon.stat_bonuses.to_hit_bonus_per_stat_point
    } else {
        0.0
    }
}

/// An error for when a weapon is currently unusable.
pub enum WeaponUnusableError {
    /// A stat requirement is not met.
    StatRequirementNotMet(WeaponStatRequirement),
    /// The weapon is outside its usable range.
    OutsideUsableRange {
        usable: RangeInclusive<CombatRange>,
        actual: CombatRange,
    },
}

/// Applies modifications to the provided damage based on the weapon's stat requirements and the entity's stats.
fn apply_stat_requirements_to_damage_range(
    range: RangeInclusive<f32>,
    weapon: &Weapon,
    entity: Entity,
    world: &World,
) -> Result<RangeInclusive<f32>, WeaponUnusableError> {
    let mut new_range = range;
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
                    new_range = apply_damage_adjustments(new_range, adjustments, 1);
                }
                WeaponStatRequirementNotMetBehavior::AdjustmentsPerPointBelowMin(adjustments) => {
                    new_range = apply_damage_adjustments(new_range, adjustments, points_below_min);
                }
            }
        }
    }

    Ok(new_range)
}

fn apply_damage_adjustments(
    range: RangeInclusive<f32>,
    adjustments: &[WeaponPerformanceAdjustment],
    times: u32,
) -> RangeInclusive<f32> {
    let mut new_range = range;
    for adjustment in adjustments {
        if let WeaponPerformanceAdjustment::Damage(damage_adjustment) = adjustment {
            new_range = apply_damage_adjustment(new_range, damage_adjustment, times);
        }
    }

    new_range
}

fn apply_damage_adjustment(
    range: RangeInclusive<f32>,
    adjustment: &WeaponDamageAdjustment,
    times: u32,
) -> RangeInclusive<f32> {
    match adjustment {
        WeaponDamageAdjustment::NewRange(new_range) => {
            *new_range.start() as f32..=*new_range.end() as f32
        }
        WeaponDamageAdjustment::Set(x) => *x as f32..=*x as f32,
        WeaponDamageAdjustment::Add(x) => range.add(*x as f32 * times as f32),
        WeaponDamageAdjustment::Multiply(x) => range.mult(x * times as f32),
        WeaponDamageAdjustment::Min => *range.start()..=*range.start(),
        WeaponDamageAdjustment::Max => *range.end()..=*range.end(),
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
