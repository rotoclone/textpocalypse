use bevy_ecs::prelude::*;

use crate::component::AmmoCaliber;

/// Component for bullets that can be fired by firearms.
#[derive(Component)]
pub struct Bullet {
    /// The caliber of the bullet
    pub caliber: AmmoCaliber,
}
