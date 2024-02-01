use bevy_ecs::prelude::*;
use rand_distr::Distribution;
use std::fmt::Display;

use strum::EnumIter;

use crate::resource::BodyPartWeights;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
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
}

impl Display for BodyPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BodyPart::Head => "head",
            BodyPart::Torso => "torso",
            BodyPart::LeftArm => "left arm",
            BodyPart::RightArm => "right arm",
            BodyPart::LeftHand => "left hand",
            BodyPart::RightHand => "right hand",
            BodyPart::LeftLeg => "left leg",
            BodyPart::RightLeg => "right leg",
            BodyPart::LeftFoot => "left foot",
            BodyPart::RightFoot => "right foot",
        };

        string.fmt(f)
    }
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
