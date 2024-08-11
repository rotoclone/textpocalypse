use std::collections::HashMap;

use bevy_ecs::prelude::*;

use super::Stat;

/// Keeps track of the number of times an entity's stats have been used in checks recently.
#[derive(Component)]
pub struct CheckHistory(HashMap<Stat, u32>);

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
                    *count = count.saturating_add(1);
                })
                .or_insert(1);

            // decrease count for all other stats
            for (_, v) in check_history.0.iter_mut().filter(|(k, _)| *k != stat) {
                *v = v.saturating_sub(1);
            }
        } else {
            world
                .entity_mut(entity)
                .insert(CheckHistory([(stat.clone(), 1)].into()));
        }
    }

    /// Gets the number of recent checks an entity has done against a stat
    pub fn get_count(stat: &Stat, entity: Entity, world: &World) -> u32 {
        if let Some(check_history) = world.get::<CheckHistory>(entity) {
            *check_history.0.get(stat).unwrap_or(&0)
        } else {
            0
        }
    }
}
