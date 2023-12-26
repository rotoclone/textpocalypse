use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{input_parser::find_parsers_relevant_for, ActionDescription};

#[derive(Debug, Clone)]
pub struct HelpDescription {
    /// Descriptions of the actions that can be performed.
    pub actions: Vec<ActionDescription>,
}

impl HelpDescription {
    /// Creates a help description for the provided entity.
    pub fn for_entity(entity: Entity, world: &World) -> HelpDescription {
        HelpDescription {
            actions: build_available_action_descriptions(entity, world),
        }
    }
}

/// Builds a list of descriptions of actions an entity can perform.
fn build_available_action_descriptions(
    looking_entity: Entity,
    world: &World,
) -> Vec<ActionDescription> {
    find_parsers_relevant_for(looking_entity, world)
        .flat_map(|p| p.get_input_formats())
        .unique()
        .map(|format| ActionDescription { format })
        .collect()
}
