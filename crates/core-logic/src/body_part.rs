use bevy_ecs::prelude::*;
use rand::seq::SliceRandom;

use strum::EnumIter;

/// The body parts an entity has.
#[derive(Component)]
pub struct BodyParts(pub Vec<BodyPart>);

/// A single body part of an entity.
pub struct BodyPart {
    /// The display name of the body part.
    pub name: String,
    /// The type of body part this is.
    pub body_part_type: BodyPartType,
    /// Amount to multiply damage by for attacks that hit this body part.
    pub damage_multiplier: f32,
}

/// Defines the different types of body part.
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)] //TODO remove EnumIter?
pub enum BodyPartType {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftHand,
    RightHand,
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
    Custom(String),
}

impl BodyPart {
    /// Gets a random body part from the provided entity, if it has any body parts.
    pub fn random_weighted(entity: Entity, world: &World) -> Option<&BodyPart> {
        let mut rng = rand::thread_rng();
        world
            .get::<BodyParts>(entity)
            //TODO allow different weights
            .map(|b| b.0.choose(&mut rng))
            .unwrap_or_default()
    }
}
