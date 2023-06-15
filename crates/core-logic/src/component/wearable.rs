use std::collections::HashSet;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{format_list, AttributeDescription, BodyPart};

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

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
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let mut descs = Vec::new();
        if let Some(wearable) = world.get::<Wearable>(entity) {
            let body_parts = wearable
                .body_parts
                .iter()
                .map(|part| part.to_string())
                .sorted()
                .collect::<Vec<String>>();
            descs.push(AttributeDescription::does(format!(
                "covers the {}",
                format_list(&body_parts)
            )));

            if detail_level >= AttributeDetailLevel::Advanced {
                descs.push(AttributeDescription::has(format!(
                    "a thickness of {}",
                    wearable.thickness
                )));
            }
        }

        descs
    }
}

impl DescribeAttributes for Wearable {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(WearableAttributeDescriber)
    }
}
