use bevy_ecs::prelude::*;

use crate::{
    component::{
        AmmoCaliber, AttributeDescriber, AttributeDetailLevel, DescribeAttributes,
        SectionAttributeDescription,
    },
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    AttributeDescription, AttributeSection, AttributeSectionName,
};

/// Component for bullets that can be fired by firearms.
#[derive(Component)]
pub struct Bullet {
    /// The caliber of the bullet
    pub caliber: AmmoCaliber,
}

impl DescribeAttributes for Bullet {
    fn get_attribute_describer() -> Box<dyn AttributeDescriber> {
        Box::new(BulletAttributeDescriber)
    }
}

/// Describes a bullet.
#[derive(Debug)]
struct BulletAttributeDescriber;

impl AttributeDescriber for BulletAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let Some(bullet) = world.get::<Bullet>(entity) else {
            return Vec::new();
        };

        vec![AttributeDescription::Section(AttributeSection {
            name: AttributeSectionName::Bullet,
            attributes: vec![SectionAttributeDescription {
                name: "Caliber".to_string(),
                description: AmmoCaliberNameCatalog::get_value(&bullet.caliber, world),
            }],
        })]
    }
}
