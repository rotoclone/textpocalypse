use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{resource::FluidNames, AttributeDescription};

use super::{
    AttributeDescriber, AttributeDetailLevel, DescribeAttributes, Fluid, OpenState, Volume,
};

/// Fluid contents of an entity.
#[derive(Component)]
pub struct FluidContainer {
    /// The contained fluid.
    pub contents: Option<Fluid>,
    /// The maximum volume of fluid this container can hold, if it is limited.
    pub volume: Option<Volume>,
}

impl FluidContainer {
    /// Creates an empty fluid container that can hold an infinite amount of fluid.
    pub fn new_infinite() -> FluidContainer {
        FluidContainer {
            contents: None,
            volume: None,
        }
    }

    /// Creates an empty fluid container.
    pub fn new(volume: Option<Volume>) -> FluidContainer {
        FluidContainer {
            contents: None,
            volume,
        }
    }

    /// Gets the total amount of volume of fluid in the container.
    pub fn get_used_volume(&self) -> Volume {
        self.contents
            .as_ref()
            .map(|fluid| fluid.get_total_volume())
            .unwrap_or(Volume(0.0))
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

            let fluid_names = world.resource::<FluidNames>();

            let fluid_names_to_volumes = container
                .contents
                .as_ref()
                .map(|f| &f.contents)
                .unwrap_or(&HashMap::new())
                .iter()
                .into_group_map_by(|(fluid_type, _)| fluid_names.for_fluid(fluid_type))
                .into_iter()
                .map(|(name, fluids)| {
                    let total_volume = fluids
                        .into_iter()
                        .map(|(_, volume)| volume)
                        .cloned()
                        .sum::<Volume>();
                    (name, total_volume)
                })
                .collect::<HashMap<String, Volume>>();

            let used_volume = container.get_used_volume();

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

                //TODO sort constituent fluids so they appear in a consistent order
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
