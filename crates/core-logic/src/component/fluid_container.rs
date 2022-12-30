use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{get_name, get_volume, AttributeDescription};

use super::{AttributeDescriber, AttributeDetailLevel, DescribeAttributes, OpenState, Volume};

/// Fluid entities contained within an entity.
#[derive(Component)]
pub struct FluidContainer {
    /// The contained entities.
    pub entities: HashSet<Entity>,
    /// The maximum volume of fluid this container can hold, if it is limited.
    pub volume: Option<Volume>,
}

impl FluidContainer {
    /// Creates an empty fluid container that can hold an infinite amount of fluid.
    pub fn new_infinite() -> FluidContainer {
        FluidContainer {
            entities: HashSet::new(),
            volume: None,
        }
    }

    /// Creates an empty fluid container.
    pub fn new(volume: Option<Volume>) -> FluidContainer {
        FluidContainer {
            entities: HashSet::new(),
            volume,
        }
    }
}

/// Describes the fluid contents of an entity.
#[derive(Debug)]
struct FluidContainerAttributeDescriber;

impl AttributeDescriber for FluidContainerAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(container) = world.get::<FluidContainer>(entity) {
            if let Some(open_state) = world.get::<OpenState>(entity) {
                if !open_state.is_open {
                    return Vec::new();
                }
            }

            let fluid_names_to_volumes = container
                .entities
                .iter()
                .into_group_map_by(|e| {
                    get_name(**e, world).unwrap_or_else(|| "an unknown fluid".to_string())
                })
                .into_iter()
                .map(|(name, entities)| {
                    let total_volume = entities
                        .into_iter()
                        .map(|e| get_volume(*e, world))
                        .sum::<Volume>();
                    (name, total_volume)
                })
                .collect::<HashMap<String, Volume>>();

            let used_volume = fluid_names_to_volumes.values().cloned().sum::<Volume>();

            let mut descriptions = Vec::new();

            if let Some(volume) = &container.volume {
                descriptions.push(AttributeDescription::does(format!(
                    "can hold {volume:.2} L of fluid"
                )));
            }

            if fluid_names_to_volumes.is_empty() {
                descriptions.push(AttributeDescription::is("empty".to_string()));
            } else if fluid_names_to_volumes.len() == 1 {
                // unwrap is safe because we've checked that there's at least one element
                let (fluid_name, fluid_volume) = fluid_names_to_volumes.iter().next().unwrap();
                descriptions.push(AttributeDescription::does(format!(
                    "contains {fluid_volume:.2} L of {fluid_name}"
                )));
            } else {
                let fluid_names_to_fractions = fluid_names_to_volumes
                    .into_iter()
                    .map(|(n, v)| (n, v / used_volume.clone()))
                    .collect::<HashMap<String, f32>>();

                let fluid_description = fluid_names_to_fractions
                    .iter()
                    .map(|(name, fraction)| format!("{:.0}% {}", fraction * 100.0, name))
                    .join(", ");

                descriptions.push(AttributeDescription::does(format!(
                    "contains {used_volume:.2} L of a combination of {fluid_description}"
                )));
            }

            return descriptions;
        }

        Vec::new()
    }
}

impl DescribeAttributes for FluidContainer {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(FluidContainerAttributeDescriber)
    }
}
