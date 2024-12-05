use std::collections::HashMap;

use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

use crate::{body_part::BodyPartType, NameWithArticle};

/// Map of body part types to their display names.
#[derive(Resource)]
pub struct BodyPartTypeNameCatalog {
    standard: HashMap<BodyPartType, NameWithArticle>,
    custom: HashMap<String, NameWithArticle>,
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
    pub fn get_name(body_part_type: &BodyPartType, world: &World) -> NameWithArticle {
        world
            .resource::<BodyPartTypeNameCatalog>()
            .get(body_part_type)
    }

    /// Sets the name of the provided body part type.
    pub fn set(&mut self, body_part_type: &BodyPartType, name: NameWithArticle) {
        match body_part_type {
            BodyPartType::Custom(id) => self.custom.insert(id.clone(), name),
            _ => self.standard.insert(body_part_type.clone(), name),
        };
    }

    /// Determines the name for the provided body part type.
    pub fn get(&self, body_part_type: &BodyPartType) -> NameWithArticle {
        match body_part_type {
            BodyPartType::Custom(id) => self.custom.get(id),
            _ => self.standard.get(body_part_type),
        }
        .cloned()
        .unwrap_or_else(|| NameWithArticle::an("unknown body part"))
    }
}

/// Builds the default display names of standard body part types.
fn build_standard_names() -> HashMap<BodyPartType, NameWithArticle> {
    BodyPartType::iter()
        .filter_map(|body_part_type| {
            get_default_name(&body_part_type).map(|name| (body_part_type, name))
        })
        .collect()
}

/// Gets the default display name of a body part type.
fn get_default_name(body_part_type: &BodyPartType) -> Option<NameWithArticle> {
    match body_part_type {
        BodyPartType::Head => Some(NameWithArticle::a("head")),
        BodyPartType::Torso => Some(NameWithArticle::a("torso")),
        BodyPartType::LeftArm => Some(NameWithArticle::a("left arm")),
        BodyPartType::RightArm => Some(NameWithArticle::a("right arm")),
        BodyPartType::LeftHand => Some(NameWithArticle::a("left hand")),
        BodyPartType::RightHand => Some(NameWithArticle::a("right hand")),
        BodyPartType::LeftLeg => Some(NameWithArticle::a("left leg")),
        BodyPartType::RightLeg => Some(NameWithArticle::a("right leg")),
        BodyPartType::LeftFoot => Some(NameWithArticle::a("left foot")),
        BodyPartType::RightFoot => Some(NameWithArticle::a("right foot")),
        BodyPartType::Custom(_) => None,
    }
}
