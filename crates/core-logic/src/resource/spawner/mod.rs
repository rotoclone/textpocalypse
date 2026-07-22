use bevy_ecs::prelude::*;

mod bullet_casing_spawner;
pub use bullet_casing_spawner::BulletCasingSpawner;

/// Registers all the spawners with their default spawn functions.
pub fn register_spawners(world: &mut World) {
    BulletCasingSpawner::register(world);
}

/// Trait for resources that spawn entities.
pub trait Spawner {
    /// Context passed to the spawn function.
    type Context;

    /// Registers the spawner with its default spawn function.
    fn register(world: &mut World);

    /// Spawns the thing and puts it in `location`.
    /// May panic if `location` isn't a container.
    fn spawn(context: Self::Context, location: Entity, world: &mut World);
}
