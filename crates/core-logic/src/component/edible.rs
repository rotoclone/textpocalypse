use bevy_ecs::prelude::*;

use crate::AttributeDescription;

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes};

/// Describes the nutritional value of an entity.
#[derive(Component)]
pub struct Edible {
    /// How many calories the entity is.
    pub calories: u16,
}

/// Describes the nutritional value of an entity.
#[derive(Debug)]
struct EdibleAttributeDescriber;

impl AttributeDescriber for EdibleAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if detail_level >= AttributeDetailLevel::Advanced {
            if let Some(edible) = world.get::<Edible>(entity) {
                return vec![AttributeDescription::does(format!(
                    "contains {} calories",
                    edible.calories
                ))];
            }
        }

        Vec::new()
    }
}

impl DescribeAttributes for Edible {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(EdibleAttributeDescriber)
    }
}
