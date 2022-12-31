use bevy_ecs::prelude::*;

use crate::notification::NotificationHandlers;

mod fluid_hydration_factors;
pub use fluid_hydration_factors::FluidHydrationFactors;

mod fluid_names;
pub use fluid_names::FluidNames;

/// Inserts all the resources into the world.
pub fn insert_resources(world: &mut World) {
    world.insert_resource(FluidHydrationFactors::new());
    world.insert_resource(FluidNames::new());
}

/// Registers notification handlers related to resources.
pub fn register_resource_handlers(world: &mut World) {
    NotificationHandlers::add_handler(fluid_hydration_factors::increase_hydration_on_drink, world);
}
