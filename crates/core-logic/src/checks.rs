use bevy_ecs::prelude::*;
use log::{debug, info};
use rand::Rng;
use rand_distr::StandardNormal;

use crate::component::{Attribute, Attributes, Skill, Skills, Stats};

const STANDARD_DEVIATION: f64 = 5.0;

/// The difficulty of a check.
#[derive(Clone, Copy, Debug)]
pub struct CheckDifficulty {
    /// The minimum result required to pass the check.
    target: u32,
    /// If the result is below this, it will be considered an extreme failure.
    extreme_failure_threshold: u32,
    /// If the result is above this, it will be considered an extreme success.
    extreme_success_threshold: u32,
}

impl CheckDifficulty {
    /// Creates a difficulty with the provided target.
    pub fn new(target: u32) -> CheckDifficulty {
        let extreme_failure_threshold = if STANDARD_DEVIATION as u32 > target {
            0
        } else {
            target - STANDARD_DEVIATION as u32
        };

        CheckDifficulty {
            target,
            extreme_failure_threshold,
            extreme_success_threshold: target + STANDARD_DEVIATION as u32,
        }
    }

    /// For trivially easy checks.
    pub fn trivial() -> CheckDifficulty {
        CheckDifficulty {
            target: 1,
            extreme_failure_threshold: 0,
            extreme_success_threshold: 2,
        }
    }

    /// For easy checks.
    pub fn easy() -> CheckDifficulty {
        CheckDifficulty {
            target: 4,
            extreme_failure_threshold: 2,
            extreme_success_threshold: 8,
        }
    }

    /// For moderately difficult checks.
    pub fn moderate() -> CheckDifficulty {
        CheckDifficulty {
            target: 7,
            extreme_failure_threshold: 3,
            extreme_success_threshold: 11,
        }
    }

    /// For difficult checks.
    pub fn hard() -> CheckDifficulty {
        CheckDifficulty {
            target: 10,
            extreme_failure_threshold: 6,
            extreme_success_threshold: 20,
        }
    }

    /// For very difficult checks.
    pub fn very_hard() -> CheckDifficulty {
        CheckDifficulty {
            target: 13,
            extreme_failure_threshold: 6,
            extreme_success_threshold: 26,
        }
    }

    /// For extremely difficult checks.
    pub fn extreme() -> CheckDifficulty {
        CheckDifficulty {
            target: 16,
            extreme_failure_threshold: 8,
            extreme_success_threshold: 32,
        }
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

impl Attributes {
    /// Performs a check against the provided attribute.
    fn check(&self, attribute: Attribute, difficulty: CheckDifficulty) -> CheckResult {
        check_normal(&format!("{attribute:?}"), self.get(attribute), difficulty)
    }
}

impl Skills {
    /// Performs a check against the provided skill.
    fn check(&self, skill: Skill, difficulty: CheckDifficulty) -> CheckResult {
        check_normal(&format!("{skill:?}"), self.get(skill), difficulty)
    }
}

/// Performs a check with the provided difficulty.
///
/// The total is generated by sampling from a normal distribution centered on the stat value.
fn check_normal(stat_name: &str, stat_value: u32, difficulty: CheckDifficulty) -> CheckResult {
    // this will generate a float from a normal distribution centered around 0 with a standard deviation of 1
    let raw_total: f64 = rand::thread_rng().sample(StandardNormal);

    // this transforms the value so it's like it came from a normal distribution with a mean of the stat value and a different standard deviation
    let float_total = (raw_total * STANDARD_DEVIATION) + stat_value as f64;

    let total = float_total.round().clamp(0.0, u32::MAX.into()) as u32;

    debug!(
        "{} check: stat value {}, difficulty {}, total {}",
        stat_name, stat_value, difficulty.target, total
    );

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

impl Stats {
    /// Performs a check against an attribute on the provided entity.
    pub fn check_attribute(
        entity: Entity,
        attribute: Attribute,
        difficulty: CheckDifficulty,
        world: &World,
    ) -> CheckResult {
        if let Some(stats) = world.get::<Stats>(entity) {
            stats.attributes.check(attribute, difficulty)
        } else {
            // the entity doesn't have stats, so they fail all checks
            CheckResult::ExtremeFailure
        }
    }

    /// Performs a check against a skill on the provided entity.
    pub fn check_skill(
        entity: Entity,
        skill: Skill,
        difficulty: CheckDifficulty,
        world: &World,
    ) -> CheckResult {
        if let Some(stats) = world.get::<Stats>(entity) {
            stats.skills.check(skill, difficulty)
        } else {
            // the entity doesn't have stats, so they fail all checks
            CheckResult::ExtremeFailure
        }
    }
}
