use bevy_ecs::prelude::*;
use log::debug;
use rand::Rng;
use rand_distr::StandardNormal;

use crate::{
    component::{Stat, Stats},
    CheckHistory, IntegerExtensions, Notification, Xp, XpAwardNotification,
};

/// The amount of XP to award for a regular check.
pub const STANDARD_CHECK_XP: Xp = Xp(10);

/// The standard deviation of rolls for checks
const STANDARD_DEVIATION: f32 = 5.0;
/// The fraction of the target number that produces the bounds of non-extreme results.
/// For example, if the target is 10, and this is 0.5, then a result of less than 5 is an extreme failure, and more than 15 is an extreme success.
const EXTREME_THRESHOLD_FRACTION: f32 = 0.5;

/// Amount to multiply base XP by when failing a check
const FAILURE_XP_MULT: f32 = 1.0;
/// Amount to multiply base XP by when succeeding at a check
const SUCCESS_XP_MULT: f32 = 0.8;

/// Amount to multiply base XP by when similar checks are performed multiple times in a row.
const REPEATED_CHECK_XP_MULTIPLIER: f32 = 0.9;

/// Modifications to be applied to a check.
#[derive(Clone, Copy, Debug)]
pub struct CheckModifiers {
    /// Any value to be added to the value of the stat before the check.
    pub value_modifier: Option<f32>,
    /// Any value to be added to the result of the check before determining success.
    pub result_modifier: Option<f32>,
}

impl CheckModifiers {
    /// Creates modifiers that do nothing.
    pub fn none() -> CheckModifiers {
        CheckModifiers {
            value_modifier: None,
            result_modifier: None,
        }
    }

    /// Creates modifiers that change the value of the stat before the check.
    pub fn modify_value(value_modifier: f32) -> CheckModifiers {
        CheckModifiers {
            value_modifier: Some(value_modifier),
            result_modifier: None,
        }
    }

    /// Creates modifiers that change the result of the check before determining success.
    pub fn modify_result(result_modifier: f32) -> CheckModifiers {
        CheckModifiers {
            value_modifier: None,
            result_modifier: Some(result_modifier),
        }
    }
}

/// Generates a total for a check using the provided stat value.
///
/// The total is generated by sampling from a normal distribution centered on the stat value.
fn roll_normal(stat: &Stat, raw_stat_value: f32, modifiers: CheckModifiers) -> u16 {
    // this will generate a float from a normal distribution centered around 0 with a standard deviation of 1
    let raw_total: f32 = rand::thread_rng().sample(StandardNormal);

    let modified_stat_value = if let Some(value_modifier) = modifiers.value_modifier {
        raw_stat_value + value_modifier
    } else {
        raw_stat_value
    };

    // this transforms the value so it's like it came from a normal distribution with a mean of the stat value and a different standard deviation
    let float_total = (raw_total * STANDARD_DEVIATION) + modified_stat_value;
    let modified_float_total = if let Some(result_modifier) = modifiers.result_modifier {
        float_total + result_modifier
    } else {
        float_total
    };

    let total = modified_float_total.round().clamp(0.0, u16::MAX.into()) as u16;

    debug!(
        "{:?} roll: value {} (raw {}), total {} (raw {})",
        stat, modified_stat_value, raw_stat_value, total, modified_float_total
    );

    total
}

/// The difficulty of a check.
#[derive(Clone, Copy, Debug)]
pub struct CheckDifficulty {
    /// The minimum result required to pass the check.
    target: u16,
    /// If the result is below this, it will be considered an extreme failure.
    extreme_failure_threshold: u16,
    /// If the result is above this, it will be considered an extreme success.
    extreme_success_threshold: u16,
}

impl CheckDifficulty {
    /// Creates a difficulty with the provided target.
    pub fn new(target: u16) -> CheckDifficulty {
        let (extreme_failure_threshold, extreme_success_threshold) =
            get_extreme_thresholds(target, EXTREME_THRESHOLD_FRACTION);

        CheckDifficulty {
            target,
            extreme_failure_threshold,
            extreme_success_threshold,
        }
    }

    /// For trivially easy checks.
    pub fn trivial() -> CheckDifficulty {
        Self::new(1)
    }

    /// For easy checks.
    pub fn easy() -> CheckDifficulty {
        Self::new(4)
    }

    /// For moderately difficult checks.
    pub fn moderate() -> CheckDifficulty {
        Self::new(7)
    }

    /// For difficult checks.
    pub fn hard() -> CheckDifficulty {
        Self::new(10)
    }

    /// For very difficult checks.
    pub fn very_hard() -> CheckDifficulty {
        Self::new(13)
    }

    /// For extremely difficult checks.
    pub fn extreme() -> CheckDifficulty {
        Self::new(16)
    }
}

/// Gets the extreme failure and extreme success thresholds for the provided target value.
fn get_extreme_thresholds(target: u16, extreme_threshold_fraction: f32) -> (u16, u16) {
    let extreme_threshold_amount =
        ((f32::from(target) * extreme_threshold_fraction).round() as u16).max(1);

    (
        target.saturating_sub(extreme_threshold_amount),
        target.saturating_add(extreme_threshold_amount),
    )
}

/// Performs a check with the provided difficulty.
fn check(
    stat: &Stat,
    stat_value: f32,
    modifiers: CheckModifiers,
    difficulty: CheckDifficulty,
) -> CheckResult {
    let total = roll_normal(stat, stat_value, modifiers);

    if total < difficulty.extreme_failure_threshold {
        CheckResult::ExtremeFailure
    } else if total < difficulty.target {
        CheckResult::Failure
    } else if total > difficulty.extreme_success_threshold {
        CheckResult::ExtremeSuccess
    } else {
        CheckResult::Success
    }
}

/// Describes a participant in a versus check.
pub struct VsParticipant {
    /// The entity participanting in the check
    pub entity: Entity,
    /// The stat the entity is using for the check
    pub stat: Stat,
    /// Any modifiers to apply to this entity's side of the check
    pub modifiers: CheckModifiers,
}

/// Identifies a participant in a versus check.
pub enum VsParticipantType {
    /// The first participant.
    First,
    /// The second participant.
    Second,
}

/// Describes parameters for a versus check.
pub struct VsCheckParams {
    /// Which participant wins if both of them have the same result
    pub winner_on_tie: VsParticipantType,
    /// If the results differ by more than this fraction of the second participant's result, it will be considered an extreme failure for one participant and an extreme success for the other.
    pub extreme_threshold_fraction: f32,
    /// The base amount of XP to award to entities involved in the check.
    pub base_xp: Xp,
}

impl VsCheckParams {
    /// Creates params with the first participant winning ties
    pub fn first_wins_ties(base_xp: Xp) -> VsCheckParams {
        VsCheckParams {
            winner_on_tie: VsParticipantType::First,
            extreme_threshold_fraction: EXTREME_THRESHOLD_FRACTION,
            base_xp,
        }
    }

    /// Creates params with the second participant winning ties
    pub fn second_wins_ties(base_xp: Xp) -> VsCheckParams {
        VsCheckParams {
            winner_on_tie: VsParticipantType::Second,
            extreme_threshold_fraction: EXTREME_THRESHOLD_FRACTION,
            base_xp,
        }
    }
}

/// Performs a check of two stats against each other.
fn check_vs(
    stat_1: &Stat,
    stat_1_value: f32,
    modifiers_1: CheckModifiers,
    stat_2: &Stat,
    stat_2_value: f32,
    modifiers_2: CheckModifiers,
    params: VsCheckParams,
) -> (CheckResult, CheckResult) {
    let total_1 = roll_normal(stat_1, stat_1_value, modifiers_1);
    let total_2 = roll_normal(stat_2, stat_2_value, modifiers_2);
    let (extreme_failure_threshold, extreme_success_threshold) =
        get_extreme_thresholds(total_2, params.extreme_threshold_fraction);

    if total_1 == total_2 {
        match params.winner_on_tie {
            VsParticipantType::First => (CheckResult::Success, CheckResult::Failure),
            VsParticipantType::Second => (CheckResult::Failure, CheckResult::Success),
        }
    } else if total_1 < extreme_failure_threshold {
        (CheckResult::ExtremeFailure, CheckResult::ExtremeSuccess)
    } else if total_1 < total_2 {
        (CheckResult::Failure, CheckResult::Success)
    } else if total_1 > extreme_success_threshold {
        (CheckResult::ExtremeSuccess, CheckResult::ExtremeFailure)
    } else {
        (CheckResult::Success, CheckResult::Failure)
    }
}

/// The result of performing a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// The total didn't even come close.
    ExtremeFailure,
    /// The total was too low, but not by a lot.
    Failure,
    /// The total was high enough, but not super high.
    Success,
    /// The total was way higher than needed.
    ExtremeSuccess,
}

impl CheckResult {
    /// Determines whether the check succeeded at all.
    pub fn succeeded(&self) -> bool {
        match self {
            CheckResult::Success | CheckResult::ExtremeSuccess => true,
            CheckResult::Failure | CheckResult::ExtremeFailure => false,
        }
    }
}

impl Stats {
    /// Performs a check against a stat on the provided entity.
    pub fn check<T: Into<Stat> + Clone>(
        entity: Entity,
        stat: T,
        modifiers: CheckModifiers,
        difficulty: CheckDifficulty,
        base_xp: Xp,
        world: &mut World,
    ) -> CheckResult {
        if let Some(stats) = world.get::<Stats>(entity) {
            let result = check(
                &stat.clone().into(),
                stat.clone().into().get_value(stats, world),
                modifiers,
                difficulty,
            );
            award_xp(entity, &stat.into(), result, base_xp, world);
            result
        } else {
            // the entity doesn't have stats, so they fail all checks
            CheckResult::ExtremeFailure
        }
    }

    /// Performs checks for two entities' stats against each other.
    ///
    /// Extreme success/failure thresholds will be determined based on the second participant's result.
    /// For example, if the second participant gets a result of 10, and the extreme threshold fraction is 0.5, then extreme results will occur for first participant results of less than 5 and greater than 15.
    pub fn check_vs(
        participant_1: VsParticipant,
        participant_2: VsParticipant,
        params: VsCheckParams,
        world: &mut World,
    ) -> (CheckResult, CheckResult) {
        let entity_1_stats = world.get::<Stats>(participant_1.entity);
        let entity_2_stats = world.get::<Stats>(participant_2.entity);
        match (entity_1_stats, entity_2_stats) {
            (Some(stats_1), Some(stats_2)) => {
                let base_xp = params.base_xp;
                let (result_1, result_2) = check_vs(
                    &participant_1.stat,
                    participant_1.stat.get_value(stats_1, world),
                    participant_1.modifiers,
                    &participant_2.stat,
                    participant_2.stat.get_value(stats_2, world),
                    participant_2.modifiers,
                    params,
                );
                award_xp(
                    participant_1.entity,
                    &participant_1.stat,
                    result_1,
                    base_xp,
                    world,
                );
                award_xp(
                    participant_2.entity,
                    &participant_2.stat,
                    result_2,
                    base_xp,
                    world,
                );
                (result_1, result_2)
            }
            // entities that don't have stats fail all checks
            (Some(_), None) => (CheckResult::ExtremeSuccess, CheckResult::ExtremeFailure),
            (None, Some(_)) => (CheckResult::ExtremeFailure, CheckResult::ExtremeSuccess),
            (None, None) => (CheckResult::ExtremeFailure, CheckResult::ExtremeFailure),
        }
    }
}

/// Gives an entity XP for a check with the given result.
fn award_xp(entity: Entity, stat: &Stat, result: CheckResult, base_xp: Xp, world: &mut World) {
    let repeat_mult =
        REPEATED_CHECK_XP_MULTIPLIER.powf(CheckHistory::get_repetition_factor(stat, entity, world));
    let xp_mult = match result {
        CheckResult::Failure => FAILURE_XP_MULT,
        CheckResult::Success => SUCCESS_XP_MULT,
        _ => 0.0,
    };

    let xp = Xp(base_xp.0.mul_and_round(xp_mult * repeat_mult));

    Notification::send_no_contents(
        XpAwardNotification {
            entity,
            xp_to_add: xp,
        },
        world,
    );

    CheckHistory::log(stat, entity, world);
}
