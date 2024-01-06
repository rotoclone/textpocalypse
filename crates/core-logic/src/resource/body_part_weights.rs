use bevy_ecs::prelude::*;
use rand::distributions::WeightedIndex;
use strum::IntoEnumIterator;

use crate::BodyPart;

#[derive(Resource)]
pub struct BodyPartWeights {
    pub body_parts: Vec<BodyPart>,
    pub dist: WeightedIndex<f32>,
}

impl BodyPartWeights {
    /// Creates the default body part weights.
    pub fn new() -> BodyPartWeights {
        let mut body_parts = Vec::new();
        let mut weights = Vec::new();
        for body_part in BodyPart::iter() {
            let weight = match body_part {
                BodyPart::Head => 0.15,
                BodyPart::Torso => 0.53,
                BodyPart::LeftArm => 0.05,
                BodyPart::RightArm => 0.05,
                BodyPart::LeftHand => 0.03,
                BodyPart::RightHand => 0.03,
                BodyPart::LeftLeg => 0.05,
                BodyPart::RightLeg => 0.05,
                BodyPart::LeftFoot => 0.03,
                BodyPart::RightFoot => 0.03,
            };
            weights.push(weight);
            body_parts.push(body_part);
        }

        BodyPartWeights {
            body_parts,
            dist: WeightedIndex::new(weights).expect("body part weights should be valid"),
        }
    }
}
