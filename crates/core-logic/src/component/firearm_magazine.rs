use bevy_ecs::prelude::*;

use crate::{
    component::{
        AmmoCaliber, AttributeDescriber, AttributeDetailLevel, DescribeAttributes, ParseCustomInput,
    },
    input_parser::InputParser,
    AttributeDescription,
};

/// Component for entities that can be loaded into firearms.
#[derive(Component)]
pub struct FirearmMagazine {
    pub caliber: AmmoCaliber,
    pub max_bullets: u16,
}

impl DescribeAttributes for FirearmMagazine {
    fn get_attribute_describer() -> Box<dyn AttributeDescriber> {
        Box::new(FirearmMagazineAttributeDescriber)
    }
}

impl FirearmMagazine {
    /// Registers handlers for magazine actions.
    pub fn register_handlers(world: &mut World) {
        //TODO
    }
}

/// Describes a magazine.
#[derive(Debug)]
struct FirearmMagazineAttributeDescriber;

impl AttributeDescriber for FirearmMagazineAttributeDescriber {
    fn describe(
        &self,
        pov_entity: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        todo!() //TODO
    }
}

// TODO add handler to prevent putting too many bullets in a magazine
