use std::{cmp::Ordering, collections::HashMap};

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    action::{PourAction, PourAmount},
    get_reference_name,
    notification::{Notification, VerifyResult},
    resource::FluidNames,
    AttributeDescription, GameMessage,
};

use super::{
    AttributeDescriber, AttributeDetailLevel, DescribeAttributes, Fluid, FluidType, OpenState,
    VerifyActionNotification, Volume,
};

/// Fluid contents of an entity.
#[derive(Component)]
pub struct FluidContainer {
    /// The contained fluid.
    /// TODO make this not an option?
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

    /// Reduces the fluid in the container by the provided amount. Returns the actual removed volumes, by fluid type.
    pub fn reduce(&mut self, amount: Volume) -> HashMap<FluidType, Volume> {
        if let Some(fluid) = self.contents.as_mut() {
            let removed_fluids = fluid.reduce(amount);
            if fluid.contents.is_empty() {
                self.contents = None;
            }

            removed_fluids
        } else {
            HashMap::new()
        }
    }

    /// Adds the provided fluid amounts to this container.
    pub fn increase(&mut self, amounts: &HashMap<FluidType, Volume>) {
        let fluid = self.contents.get_or_insert_with(|| Fluid {
            contents: HashMap::new(),
        });

        fluid.increase(amounts);

        if fluid.contents.is_empty() {
            self.contents = None;
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
                    .map(|(n, v)| (n, v / used_volume))
                    .collect::<HashMap<String, f32>>();

                let fluid_description = fluid_names_to_fractions
                    .iter()
                    .sorted_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Ordering::Equal))
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

    let used_volume = container.get_used_volume();
    let source_name = get_reference_name(source, performing_entity, world);

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
        let used_volume = container.get_used_volume();
        let available_volume = *max_volume - used_volume;
        let target_name = get_reference_name(target, performing_entity, world);
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
