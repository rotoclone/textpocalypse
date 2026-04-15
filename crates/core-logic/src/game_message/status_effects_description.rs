use std::{collections::HashMap, fmt::Display};

use bevy_ecs::prelude::*;

use crate::{
    component::{StatusEffectDetails, StatusEffects},
    StatAdjustment,
};

/// The description of an entity's status effects.
#[derive(Debug, Clone)]
pub struct StatusEffectsDescription(pub Vec<StatusEffectDescription>);

/// The description of a single status effect.
#[derive(Debug, Clone)]
pub struct StatusEffectDescription {
    /// The name of the status effect
    pub name: String,
    /// Any stat adjustments applied by the status effect
    pub stat_adjustments: HashMap<StatName, Vec<StatAdjustment>>,
    /// Any other effects applied by the status effect
    pub other_effects: Vec<String>,
}

impl StatusEffectDescription {
    /// Builds a `StatusEffectDescription` from a `StatusEffectDetails`.
    fn from_details(details: &StatusEffectDetails, world: &World) -> StatusEffectDescription {
        let stat_adjustments = details
            .stat_adjustments
            .0
            .iter()
            .map(|(stat, adjustments)| (StatName(stat.get_name(world)), adjustments.clone()))
            .collect();

        StatusEffectDescription {
            name: details.name.clone(),
            stat_adjustments,
            other_effects: details.other_effects.clone(),
        }
    }
}

/// The name of a stat.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StatName(pub String);

impl Display for StatName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl StatusEffectsDescription {
    /// Creates a status effects description for the provided entity.
    pub fn for_entity(entity: Entity, world: &World) -> StatusEffectsDescription {
        StatusEffectsDescription(
            StatusEffects::get_all(entity, world)
                .iter()
                .map(|effect| StatusEffectDescription::from_details(effect, world))
                .collect(),
        )
    }
}
