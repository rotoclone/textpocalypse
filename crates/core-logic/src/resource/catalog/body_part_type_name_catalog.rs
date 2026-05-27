use std::collections::HashMap;

use bevy_ecs::prelude::*;
use core_logic_derive::CatalogBoilerplate;

use crate::{body_part::BodyPartType, resource::catalog::Catalog, NameWithArticle};

/// Map of body part types to their display names.
#[derive(Resource, CatalogBoilerplate)]
#[catalog_type(BodyPartType)]
pub struct BodyPartTypeNameCatalog {
    standard: HashMap<BodyPartType, NameWithArticle>,
    custom: HashMap<String, NameWithArticle>,
}

impl Catalog<BodyPartType> for BodyPartTypeNameCatalog {
    type V = NameWithArticle;

    fn get_default_value(thing: &BodyPartType) -> Option<Self::V> {
        match thing {
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

    fn get_not_found_value() -> Self::V {
        NameWithArticle::an("unknown body part")
    }
}
