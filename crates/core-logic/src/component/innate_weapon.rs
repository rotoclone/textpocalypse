use bevy_ecs::prelude::*;

/// Marks an entity as the weapon an entity uses when it has nothing equipped.
#[derive(Component)]
pub struct InnateWeapon;
