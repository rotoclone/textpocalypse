use std::fmt::Debug;

use bevy_ecs::prelude::*;
/* TODO remove?
#[derive(Component)]
pub struct AttributeDescribers {
    pub describers: Vec<Box<dyn AttributeDescriber>>,
}

pub trait AttributeDescriber: Send + Sync + Debug {
    /// Generates descriptions of attributes of the provided entity.
    fn describe(&self, entity: Entity, world: &World) -> Vec<AttributeDescription>;
}

#[derive(Debug)]
pub struct AttributeDescription {
    pub description: String,
}
*/
