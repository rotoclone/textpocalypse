use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::BodyPart;

/// Map of body parts to their display names.
#[derive(Resource)]
pub struct BodyPartNameCatalog {
    standard: HashMap<BodyPart, String>,
    custom: HashMap<String, String>,
}

impl BodyPartNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> BodyPartNameCatalog {
        let standard_names = build_standard_names();
        BodyPartNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
        }
    }

    /// Gets the name of the provided body part.
    pub fn get_name(body_part: &BodyPart, world: &World) -> String {
        world.resource::<BodyPartNameCatalog>().get(body_part)
    }

    /// Sets the name of the provided body part.
    pub fn set(&mut self, body_part: &BodyPart, name: String) {
        match body_part {
            BodyPart::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(body_part.clone(), name),
        };
    }

    /// Determines the name for the provided body part.
    pub fn get(&self, body_part: &BodyPart) -> String {
        match body_part {
            BodyPart::Custom(id) => self.custom.get(id),
            _ => self.standard.get(body_part),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown body part".to_string())
    }
}

/// Builds the default display names of standard body parts.
fn build_standard_names() -> HashMap<BodyPart, String> {
    BodyPart::iter()
        .filter_map(|body_part| get_default_name(&body_part).map(|name| (body_part, name)))
        .collect()
}

/// Gets the default display name of a body part.
fn get_default_name(body_part: &BodyPart) -> Option<String> {
    match body_part {
        BodyPart::Head => Some("head"),
        BodyPart::Torso => Some("torso"),
        BodyPart::LeftArm => Some("left arm"),
        BodyPart::RightArm => Some("right arm"),
        BodyPart::LeftHand => Some("left hand"),
        BodyPart::RightHand => Some("right hand"),
        BodyPart::LeftLeg => Some("left leg"),
        BodyPart::RightLeg => Some("right leg"),
        BodyPart::LeftFoot => Some("left foot"),
        BodyPart::RightFoot => Some("right foot"),
        BodyPart::Custom(_) => todo!(),
    }
    .map(|s| s.to_string())
}
