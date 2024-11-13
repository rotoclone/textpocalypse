use bevy_ecs::prelude::*;
use rand_distr::Distribution;

use strum::EnumIter;

use crate::resource::BodyPartWeights;

/// The body parts an entity has.
#[derive(Component)]
pub struct BodyParts(pub Vec<BodyPart>);

/// A body part an entity can have.
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)] //TODO remove EnumIter?
pub enum BodyPart {
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
    /// Gets a random body part, weighted by the weights defined in the provided world.
    pub fn random_weighted(world: &World) -> BodyPart {
        if let Some(default_weights) = world.get_resource::<BodyPartWeights>() {
            let mut rng = rand::thread_rng();
            default_weights.body_parts[default_weights.dist.sample(&mut rng)]
        } else {
            BodyPart::Torso
        }
    }
}
