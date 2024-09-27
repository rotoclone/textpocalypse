use std::collections::HashSet;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{format_list, AttributeDescription, BodyPart};

use super::{
    AttributeDescriber, AttributeDetailLevel, AttributeSection, AttributeSectionName,
    DescribeAttributes, SectionAttributeDescription,
};

/// An entity that can be worn.
#[derive(Component)]
pub struct Wearable {
    /// The thickness of the entity.
    pub thickness: u32,
    /// The body parts the entity covers when worn.
    pub body_parts: HashSet<BodyPart>,
}

/// Describes the wearability of an entity.
#[derive(Debug)]
struct WearableAttributeDescriber;

impl AttributeDescriber for WearableAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(wearable) = world.get::<Wearable>(entity) {
            let body_parts = wearable
                .body_parts
                .iter()
                .map(|part| part.to_string())
                .sorted()
                .collect::<Vec<String>>();

            return vec![AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Wearable,
                attributes: vec![
                    SectionAttributeDescription {
                        name: "Body parts".to_string(),
                        description: format_list(&body_parts),
                    },
                    SectionAttributeDescription {
                        name: "Thickness".to_string(),
                        description: wearable.thickness.to_string(),
                    },
                ],
            })];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Wearable {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(WearableAttributeDescriber)
    }
}
