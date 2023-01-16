use std::{cmp::Ordering, collections::HashMap};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    action::{PourAction, PourAmount},
    get_reference_name,
    notification::{Notification, VerifyResult},
    resource::FluidNameCatalog,
    AttributeDescription, GameMessage,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, DescribeAttributes, Fluid, OpenState,
    VerifyActionNotification, Volume,
};

/// Fluid contents of an entity.
#[derive(Component)]
pub struct FluidContainer {
    /// The contained fluid.
    pub contents: Fluid,
    /// The maximum volume of fluid this container can hold, if it is limited.
    pub volume: Option<Volume>,
}

impl FluidContainer {
    /// Creates an empty fluid container that can hold an infinite amount of fluid.
    pub fn new_infinite() -> FluidContainer {
        FluidContainer {
            contents: Fluid::new(),
            volume: None,
        }
    }

    /// Creates an empty fluid container.
    pub fn new(volume: Volume) -> FluidContainer {
        FluidContainer {
            contents: Fluid::new(),
            volume: Some(volume),
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

            let fluid_names = world.resource::<FluidNameCatalog>();

            let fluid_names_to_volumes = container
                .contents
                .contents
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

            let used_volume = container.contents.get_total_volume();

            let mut descriptions = Vec::new();

            let is_full;
            if let Some(volume) = container.volume {
                descriptions.push(AttributeDescription::does(format!(
                    "can hold {volume:.2} L of fluid"
                )));

                is_full = volume == used_volume;
            } else {
                is_full = false;
            }

            if fluid_names_to_volumes.is_empty() {
                descriptions.push(AttributeDescription::is("empty".to_string()));
            } else if fluid_names_to_volumes.len() == 1 {
                // unwrap is safe because we've checked that there's at least one element
                let (fluid_name, fluid_volume) = fluid_names_to_volumes.iter().next().unwrap();
                if is_full {
                    descriptions.push(AttributeDescription::is(format!("full of {fluid_name}")));
                } else {
                    descriptions.push(AttributeDescription::does(format!(
                        "contains {fluid_volume:.2} L of {fluid_name}"
                    )));
                }
            } else {
                let fluid_names_to_fractions = fluid_names_to_volumes
                    .into_iter()
                    .map(|(n, v)| (n, v / used_volume))
                    .collect::<HashMap<String, f32>>();

                let fluid_description = fluid_names_to_fractions
                    .iter()
                    .sorted_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Ordering::Equal))
                    .map(|(name, fraction)| format!("{:.0}% {}", fraction * 100.0, name))
                    .join(", ");

                if is_full {
                    descriptions.push(AttributeDescription::is(format!(
                        "full of a combination of {fluid_description}"
                    )));
                } else {
                    descriptions.push(AttributeDescription::does(format!(
                        "contains {used_volume:.2} L of a combination of {fluid_description}"
                    )));
                }
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

/// Prevents more fluid being poured out of a fluid container than it contains.
pub fn verify_source_container(
    notification: &Notification<VerifyActionNotification, PourAction>,
    world: &World,
) -> VerifyResult {
    let source = notification.contents.source;
    let amount = &notification.contents.amount;
    let performing_entity = notification.notification_type.performing_entity;

    let container = world
        .get::<FluidContainer>(source)
        .expect("source entity should be a fluid container");

    let used_volume = container.contents.get_total_volume();
    let source_name = get_reference_name(source, Some(performing_entity), world);

    if used_volume <= Volume(0.0) {
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("{source_name} is empty.")),
        );
    }

    if let PourAmount::Some(amount) = amount {
        if used_volume < *amount {
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!(
                    "{source_name} only contains {used_volume:.2}L of fluid."
                )),
            );
        }
    }

    VerifyResult::valid()
}

/// Prevents fluid containers from getting overfilled.
pub fn limit_fluid_container_contents(
    notification: &Notification<VerifyActionNotification, PourAction>,
    world: &World,
) -> VerifyResult {
    let target = notification.contents.target;
    let amount = &notification.contents.amount;
    let performing_entity = notification.notification_type.performing_entity;

    let container = world
        .get::<FluidContainer>(target)
        .expect("destination entity should be a fluid container");

    if let Some(max_volume) = &container.volume {
        let used_volume = container.contents.get_total_volume();
        let available_volume = *max_volume - used_volume;
        let target_name = get_reference_name(target, Some(performing_entity), world);
        if available_volume <= Volume(0.0) {
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!("{target_name} is full.")),
            );
        }

        if let PourAmount::Some(amount) = amount {
            if amount > &available_volume {
                return VerifyResult::invalid(
                    performing_entity,
                    GameMessage::Error(format!(
                        "{target_name} can only hold {available_volume:.2}L more."
                    )),
                );
            }
        }
    }

    VerifyResult::valid()
}
