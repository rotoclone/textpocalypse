use bevy_ecs::prelude::*;

use crate::{
    component::{Container, Description, Volume, Weight},
    find_wearing_entity, find_wielding_entity, get_volume, get_weight,
};

/// The description of a container.
#[derive(Debug, Clone)]
pub struct ContainerDescription {
    /// Descriptions of the items in the container.
    pub items: Vec<ContainerEntityDescription>,
    /// The total volume used by items in this container.
    pub used_volume: Volume,
    /// The maximum volume of items this container can hold, if it is limited.
    pub max_volume: Option<Volume>,
    /// The total weight used by items in this container.
    pub used_weight: Weight,
    /// The maximum weight of items this container can hold, if it is limited.
    pub max_weight: Option<Weight>,
}

impl ContainerDescription {
    /// Creates a container description for the provided container.
    pub fn from_container(container: &Container, world: &World) -> ContainerDescription {
        let items = container
            .entities
            .iter()
            .flat_map(|entity| ContainerEntityDescription::from_entity(*entity, world))
            .collect::<Vec<ContainerEntityDescription>>();

        let used_volume = container.used_volume(world);
        let used_weight = container.used_weight(world);

        ContainerDescription {
            items,
            used_volume,
            max_volume: container.volume,
            used_weight,
            max_weight: container.max_weight,
        }
    }
}

/// The description of an item in a container.
#[derive(Debug, Clone)]
pub struct ContainerEntityDescription {
    /// The name of the item.
    pub name: String,
    /// The volume of the item.
    pub volume: Volume,
    /// The weight of the item.
    pub weight: Weight,
    /// Whether the item is being worn.
    pub is_being_worn: bool,
    /// Whether the item is equipped.
    pub is_equipped: bool,
}

impl ContainerEntityDescription {
    /// Creates a container entity description for the provided entity.
    /// Returns `None` if the provided entity has no `Description` component.
    pub fn from_entity(entity: Entity, world: &World) -> Option<ContainerEntityDescription> {
        let entity_ref = world.entity(entity);
        let desc = entity_ref.get::<Description>()?;
        let volume = get_volume(entity, world);
        let weight = get_weight(entity, world);
        let is_being_worn = find_wearing_entity(entity, world).is_some();
        let is_equipped = find_wielding_entity(entity, world).is_some();

        Some(ContainerEntityDescription {
            name: desc.name.clone(),
            volume,
            weight,
            is_being_worn,
            is_equipped,
        })
    }
}
