use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::body_part::BodyPartType;

/// Map of body part types to their display names.
#[derive(Resource)]
pub struct BodyPartTypeNameCatalog {
    standard: HashMap<BodyPartType, String>,
    custom: HashMap<String, String>,
}

impl BodyPartTypeNameCatalog {
    /// Creates the default catalog of names.
    pub fn new() -> BodyPartTypeNameCatalog {
        let standard_names = build_standard_names();
        BodyPartTypeNameCatalog {
            standard: standard_names,
            custom: HashMap::new(),
        }
    }

    /// Gets the name of the provided body part type.
    pub fn get_name(body_part_type: &BodyPartType, world: &World) -> String {
        world
            .resource::<BodyPartTypeNameCatalog>()
            .get(body_part_type)
    }

    /// Sets the name of the provided body part type.
    pub fn set(&mut self, body_part_type: &BodyPartType, name: String) {
        match body_part_type {
            BodyPartType::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(body_part_type.clone(), name),
        };
    }

    /// Determines the name for the provided body part type.
    pub fn get(&self, body_part_type: &BodyPartType) -> String {
        match body_part_type {
            BodyPartType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(body_part_type),
        }
        .cloned()
        .unwrap_or_else(|| "an unknown body part".to_string())
    }
}

/// Builds the default display names of standard body part types.
fn build_standard_names() -> HashMap<BodyPartType, String> {
    BodyPartType::iter()
        .filter_map(|body_part_type| {
            get_default_name(&body_part_type).map(|name| (body_part_type, name))
        })
        .collect()
}

/// Gets the default display name of a body part type.
fn get_default_name(body_part_type: &BodyPartType) -> Option<String> {
    match body_part_type {
        BodyPartType::Head => Some("head"),
        BodyPartType::Torso => Some("torso"),
        BodyPartType::LeftArm => Some("left arm"),
        BodyPartType::RightArm => Some("right arm"),
        BodyPartType::LeftHand => Some("left hand"),
        BodyPartType::RightHand => Some("right hand"),
        BodyPartType::LeftLeg => Some("left leg"),
        BodyPartType::RightLeg => Some("right leg"),
        BodyPartType::LeftFoot => Some("left foot"),
        BodyPartType::RightFoot => Some("right foot"),
        BodyPartType::Custom(_) => todo!(),
    }
    .map(|s| s.to_string())
}
