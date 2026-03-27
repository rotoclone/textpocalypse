use std::{collections::HashMap, fmt::Display};

use bevy_ecs::prelude::*;

use crate::StatAdjustment;

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

/// The name of a stat.
#[derive(Debug, Clone)]
pub struct StatName(pub String);

impl Display for StatName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl StatusEffectsDescription {
    /// Creates a status effects description for the provided entity.
    pub fn for_entity(entity: Entity, world: &World) -> StatusEffectsDescription {
        todo!() //TODO
    }
}
