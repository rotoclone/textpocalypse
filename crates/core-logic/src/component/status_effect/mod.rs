use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::component::StatAdjustments;

mod hungry;
use hungry::*;

/// Registers notification handlers related to status effects.
pub fn register_status_effect_handlers(world: &mut World) {
    Hungry::register_notification_handlers(world);
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct StatusEffectId(pub &'static str);

#[derive(Debug, Clone)]
pub struct StatusEffectDetails {
    /// The name of the status effect
    pub name: String,
    /// Any stat adjustments applied by the status effect
    pub stat_adjustments: StatAdjustments,
    /// A description of any other effects the status effect has
    pub other_effects: Vec<String>,
}

pub trait StatusEffect: Component + Sized {
    /// Adds this status effect to an entity.
    fn add_to(self, entity: Entity, world: &mut World) {
        self.on_add(entity, world);
        StatusEffects::register(entity, Self::get_id(), self.get_details(), world);

        world.entity_mut(entity).insert(self);
    }

    /// Removes this status effect from an entity.
    fn remove_from(entity: Entity, world: &mut World) {
        Self::on_remove(entity, world);
        StatusEffects::unregister(entity, Self::get_id(), world);

        world.entity_mut(entity).remove::<Self>();
    }

    /// Registers any notification handlers for this status effect.
    fn register_notification_handlers(world: &mut World);

    /// Gets the unique ID of the status effect.
    fn get_id() -> StatusEffectId;

    /// Gets a description of the status effect.
    fn get_details(&self) -> StatusEffectDetails;

    /// Performs any additional logic needed when the status effect is added to an entity.
    /// Will be called before the status effect is registered in `StatusEffects` and before the status effect component is actually added to the entity.
    fn on_add(&self, entity: Entity, world: &mut World);

    /// Performs any additional logic needed when the status effect is removed from an entity.
    /// Will be called before the status effect is unregistered in `StatusEffects` and before the status effect component is actually removed from the entity.
    fn on_remove(entity: Entity, world: &mut World);
}

/// Keeps track of the active status effects on an entity.
#[derive(Component)]
pub struct StatusEffects(HashMap<StatusEffectId, StatusEffectDetails>);

impl StatusEffects {
    /// Gets all the active status effects on an entity.
    pub fn get_all(entity: Entity, world: &World) -> Vec<&StatusEffectDetails> {
        if let Some(status_effects) = world.get::<StatusEffects>(entity) {
            status_effects.0.values().collect()
        } else {
            Vec::new()
        }
    }

    /// Registers a status effect on an entity.
    fn register(
        entity: Entity,
        id: StatusEffectId,
        details: StatusEffectDetails,
        world: &mut World,
    ) {
        todo!() //TODO
    }

    /// Unregisters a status effect from an entity.
    fn unregister(entity: Entity, id: StatusEffectId, world: &mut World) {
        todo!() //TODO
    }
}
