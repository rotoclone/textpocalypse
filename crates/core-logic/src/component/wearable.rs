use std::collections::HashSet;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    body_part::BodyPartType, format_list, resource::BodyPartTypeNameCatalog, AttributeDescription,
};

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
    pub body_parts: HashSet<BodyPartType>,
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
            let body_part_names = wearable
                .body_parts
                .iter()
                .map(|part_type| {
                    (
                        // include type for sorting purposes
                        part_type,
                        BodyPartTypeNameCatalog::get_name(part_type, world).name,
                    )
                })
                .sorted()
                .map(|(_, name)| name)
                .collect::<Vec<String>>();

            return vec![AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Wearable,
                attributes: vec![
                    SectionAttributeDescription {
                        name: "Body parts".to_string(),
                        description: format_list(&body_part_names),
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
