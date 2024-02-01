use bevy_ecs::prelude::*;

use super::Weapon;

/// The weapon an entity uses when it has nothing equipped.
#[derive(Component)]
pub struct InnateWeapon {
    /// The name of the weapon.
    pub name: String,
    /// The weapon itself.
    pub weapon: Weapon,
}
