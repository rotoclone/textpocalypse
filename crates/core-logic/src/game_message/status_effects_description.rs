use bevy_ecs::prelude::*;

use crate::component::StatusEffectDescription;

/// The description of an entity's status effects.
#[derive(Debug, Clone)]
pub struct StatusEffectsDescription(pub Vec<StatusEffectDescription>);

impl StatusEffectsDescription {
    /// Creates a vitals description for the provided vitals.
    pub fn for_entity(entity: Entity, world: &World) -> StatusEffectsDescription {
        todo!() //TODO
    }
}
