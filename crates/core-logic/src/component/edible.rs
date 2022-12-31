use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// Marks an entity as edible.
#[derive(Component)]
pub struct Edible;

/// Notes if an entity is edible.
#[derive(Debug)]
struct EdibleAttributeDescriber;

impl AttributeDescriber for EdibleAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if world.get::<Edible>(entity).is_some() {
            return vec![AttributeDescription::is("edible".to_string())];
        }

        Vec::new()
    }
}

impl DescribeAttributes for Edible {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(EdibleAttributeDescriber)
    }
}
