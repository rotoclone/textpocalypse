use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::{
    component::{CombatRange, WeaponRanges},
    Description,
};

/// A description of the ranges to combatants.
#[derive(Debug, Clone)]
pub struct RangesDescription {
    /// Descriptions of the ranges to combatants.
    pub ranges: Vec<RangeDescription>,
}

/// A description of the range to a combatant.
#[derive(Debug, Clone)]
pub struct RangeDescription {
    /// The name of the combatant.
    pub name: String,
    /// The range to the combatant.
    pub range: CombatRange,
    /// What this range means in the context of the POV entity's weapon.
    pub weapon_judgement: WeaponRangeJudgement,
}

/// What a range means in the context of a weapon.
#[derive(Debug, Clone, Copy)]
pub enum WeaponRangeJudgement {
    /// The weapon is outside its usable range.
    NotUsable(WeaponRangeJudgementReason),
    /// The weapon is inside its usable range, but outside its optimal range.
    Usable(WeaponRangeJudgementReason),
    /// The weapon is inside its optimal range.
    Optimal,
}

/// The reason why a range judgement was chosen.
#[derive(Debug, Clone, Copy)]
pub enum WeaponRangeJudgementReason {
    /// The range is longer than the ideal range.
    TooLong,
    /// The range is shorter than the ideal range.
    TooShort,
    /// The POV entity has no weapon.
    NoWeapon,
}

impl RangesDescription {
    /// Creates a ranges description from the provided combatants, judging their ranges based on the provided ranges.
    pub fn from_combatants(
        combatants: HashMap<Entity, CombatRange>,
        weapon_ranges: Option<&WeaponRanges>,
        world: &World,
    ) -> RangesDescription {
        RangesDescription {
            ranges: build_range_descriptions(combatants, weapon_ranges, world),
        }
    }
}

/// Builds a list of descriptions of ranges to combatants.
fn build_range_descriptions(
    combatants: HashMap<Entity, CombatRange>,
    weapon_ranges: Option<&WeaponRanges>,
    world: &World,
) -> Vec<RangeDescription> {
    let mut descriptions = Vec::new();
    for (entity, range) in combatants {
        descriptions.push(RangeDescription {
            name: Description::get_name(entity, world).unwrap_or_else(|| "???".to_string()),
            range,
            weapon_judgement: build_range_judgement(range, weapon_ranges),
        });
    }
    descriptions
}

/// Builds a range judgement for the provided range in the context of the provided weapon ranges.
fn build_range_judgement(
    range: CombatRange,
    weapon_ranges: Option<&WeaponRanges>,
) -> WeaponRangeJudgement {
    if let Some(weapon_ranges) = weapon_ranges {
        if weapon_ranges.optimal.contains(&range) {
            return WeaponRangeJudgement::Optimal;
        }

        let reason = if range < *weapon_ranges.optimal.start() {
            WeaponRangeJudgementReason::TooShort
        } else {
            WeaponRangeJudgementReason::TooLong
        };

        if weapon_ranges.usable.contains(&range) {
            return WeaponRangeJudgement::Usable(reason);
        }

        return WeaponRangeJudgement::NotUsable(reason);
    }

    WeaponRangeJudgement::NotUsable(WeaponRangeJudgementReason::NoWeapon)
}
