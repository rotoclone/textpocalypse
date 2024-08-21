use std::collections::HashMap;

use bevy_ecs::prelude::*;

use super::Stat;

const INCREASE_PER_CHECK: f32 = 1.5;
const DECREASE_PER_OTHER_CHECK: f32 = 1.0;

/// Keeps track of how often an entity's stats have been used in checks recently.
#[derive(Component, PartialEq, Debug)]
pub struct CheckHistory(HashMap<Stat, f32>);

impl CheckHistory {
    /// Creates an empty check history
    pub fn new() -> CheckHistory {
        CheckHistory(HashMap::new())
    }

    /// Adds a check against a stat to the history of an entity.
    pub fn log(stat: &Stat, entity: Entity, world: &mut World) {
        if let Some(mut check_history) = world.get_mut::<CheckHistory>(entity) {
            // increase count for checked stat
            check_history
                .0
                .entry(stat.clone())
                .and_modify(|count| {
                    *count += INCREASE_PER_CHECK;
                })
                .or_insert(INCREASE_PER_CHECK);

            // decrease count for all other stats
            for (_, v) in check_history.0.iter_mut().filter(|(k, _)| *k != stat) {
                *v = (*v - DECREASE_PER_OTHER_CHECK).max(0.0);
            }
        } else {
            world
                .entity_mut(entity)
                .insert(CheckHistory([(stat.clone(), INCREASE_PER_CHECK)].into()));
        }
    }

    /// Gets a number representing the frequency of recent checks an entity has done against a stat
    pub fn get_repetition_factor(stat: &Stat, entity: Entity, world: &World) -> f32 {
        if let Some(check_history) = world.get::<CheckHistory>(entity) {
            *check_history.0.get(stat).unwrap_or(&0.0)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Attribute, Skill};

    use super::*;

    #[test]
    fn log_no_history() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        CheckHistory::log(&Stat::Skill(Skill::Dodge), entity, &mut world);

        let expected_history =
            CheckHistory([(Stat::Skill(Skill::Dodge), INCREASE_PER_CHECK)].into());

        assert_eq!(Some(&expected_history), world.get::<CheckHistory>(entity));
    }

    #[test]
    fn log_empty_history() {
        let mut world = World::new();
        let entity = world.spawn(CheckHistory(HashMap::new())).id();

        CheckHistory::log(&Stat::Skill(Skill::Dodge), entity, &mut world);

        let expected_history =
            CheckHistory([(Stat::Skill(Skill::Dodge), INCREASE_PER_CHECK)].into());

        assert_eq!(Some(&expected_history), world.get::<CheckHistory>(entity));
    }

    #[test]
    fn log_non_empty_history_stat_not_in_history() {
        let mut world = World::new();
        let entity = world
            .spawn(CheckHistory(
                [
                    (Stat::Skill(Skill::Craft), 3.0),
                    (Stat::Attribute(Attribute::Strength), 1.0),
                    (Stat::Skill(Skill::Construction), 0.5),
                ]
                .into(),
            ))
            .id();

        CheckHistory::log(&Stat::Skill(Skill::Dodge), entity, &mut world);

        let expected_history = CheckHistory(
            [
                (Stat::Skill(Skill::Dodge), INCREASE_PER_CHECK),
                (Stat::Skill(Skill::Craft), 2.0),
                (Stat::Attribute(Attribute::Strength), 0.0),
                (Stat::Skill(Skill::Construction), 0.0),
            ]
            .into(),
        );

        assert_eq!(Some(&expected_history), world.get::<CheckHistory>(entity));
    }

    #[test]
    fn log_non_empty_history_stat_in_history() {
        let mut world = World::new();
        let entity = world
            .spawn(CheckHistory(
                [
                    (Stat::Skill(Skill::Dodge), 2.0),
                    (Stat::Skill(Skill::Craft), 3.0),
                    (Stat::Attribute(Attribute::Strength), 1.0),
                    (Stat::Skill(Skill::Construction), 0.5),
                ]
                .into(),
            ))
            .id();

        CheckHistory::log(&Stat::Skill(Skill::Dodge), entity, &mut world);

        let expected_history = CheckHistory(
            [
                (Stat::Skill(Skill::Dodge), 3.5),
                (Stat::Skill(Skill::Craft), 2.0),
                (Stat::Attribute(Attribute::Strength), 0.0),
                (Stat::Skill(Skill::Construction), 0.0),
            ]
            .into(),
        );

        assert_eq!(Some(&expected_history), world.get::<CheckHistory>(entity));
    }

    #[test]
    fn log_non_empty_history_stat_in_history_no_other_stats() {
        let mut world = World::new();
        let entity = world
            .spawn(CheckHistory([(Stat::Skill(Skill::Dodge), 2.0)].into()))
            .id();

        CheckHistory::log(&Stat::Skill(Skill::Dodge), entity, &mut world);

        let expected_history = CheckHistory([(Stat::Skill(Skill::Dodge), 3.5)].into());

        assert_eq!(Some(&expected_history), world.get::<CheckHistory>(entity));
    }

    #[test]
    fn get_repetition_factor_no_history() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        assert_eq!(
            0.0,
            CheckHistory::get_repetition_factor(&Stat::Skill(Skill::Dodge), entity, &world)
        );
    }

    #[test]
    fn get_repetition_factor_history_without_provided_stat() {
        let mut world = World::new();
        let entity = world
            .spawn(CheckHistory(
                [
                    (Stat::Skill(Skill::Craft), 3.0),
                    (Stat::Attribute(Attribute::Strength), 1.0),
                    (Stat::Skill(Skill::Construction), 0.5),
                ]
                .into(),
            ))
            .id();

        assert_eq!(
            0.0,
            CheckHistory::get_repetition_factor(&Stat::Skill(Skill::Dodge), entity, &world)
        );
    }

    #[test]
    fn get_repetition_factor_history_with_provided_stat() {
        let mut world = World::new();
        let entity = world
            .spawn(CheckHistory(
                [
                    (Stat::Skill(Skill::Dodge), 2.0),
                    (Stat::Skill(Skill::Craft), 3.0),
                    (Stat::Attribute(Attribute::Strength), 1.0),
                    (Stat::Skill(Skill::Construction), 0.5),
                ]
                .into(),
            ))
            .id();

        assert_eq!(
            2.0,
            CheckHistory::get_repetition_factor(&Stat::Skill(Skill::Dodge), entity, &world)
        );
    }
}
