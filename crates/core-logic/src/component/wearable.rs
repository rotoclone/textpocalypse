use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::BodyPart;

/// An entity that can be worn.
#[derive(Component)]
pub struct Wearable {
    /// The thickness of the entity.
    pub thickness: u32,
    /// THe body parts the entity covers when worn.
    pub body_parts: HashSet<BodyPart>,
}

//TODO add attribute describer
