use bevy_ecs::prelude::*;

use crate::notification::NotificationHandlers;

mod fluid_hydration_factor_catalog;
pub use fluid_hydration_factor_catalog::FluidHydrationFactorCatalog;

mod fluid_name_catalog;
pub use fluid_name_catalog::get_fluid_name;
pub use fluid_name_catalog::FluidNameCatalog;

mod fluid_density_catalog;
pub use fluid_density_catalog::get_fluid_density;
pub use fluid_density_catalog::FluidDensityCatalog;

/// Inserts all the resources into the world.
pub fn insert_resources(world: &mut World) {
    world.insert_resource(FluidHydrationFactorCatalog::new());
    world.insert_resource(FluidNameCatalog::new());
}

/// Registers notification handlers related to resources.
pub fn register_resource_handlers(world: &mut World) {
    NotificationHandlers::add_handler(
        fluid_hydration_factor_catalog::increase_hydration_on_drink,
        world,
    );
}
