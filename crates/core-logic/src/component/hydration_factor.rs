use bevy_ecs::prelude::*;

/// The amount of hydration drinking an entity provides, compared to pure water.
#[derive(Component)]
pub struct HydrationFactor(pub f32);
