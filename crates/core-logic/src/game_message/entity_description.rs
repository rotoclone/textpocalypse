use std::iter::once;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::{AttributeDescription, AttributeDetailLevel, Description, Pronouns},
    input_parser::find_parsers_relevant_for,
    ActionDescription,
};

/// The description of an entity.
#[derive(Debug, Clone)]
pub struct EntityDescription {
    /// The name of the entity.
    pub name: String,
    /// Other names for the entity.
    pub aliases: Vec<String>,
    /// The article to use when referring to the entity (usually "a" or "an").
    pub article: Option<String>,
    /// The pronouns to use when referring to the entity.
    pub pronouns: Pronouns,
    /// The description of the entity.
    pub description: String,
    /// Descriptions of dynamic attributes of the entity.
    pub attributes: Vec<AttributeDescription>,
}

impl EntityDescription {
    /// Creates an entity description for an entity from the perspective of another entity.
    pub fn for_entity(
        pov_entity: Entity,
        entity: Entity,
        desc: &Description,
        world: &World,
    ) -> EntityDescription {
        EntityDescription::for_entity_with_detail_level(
            pov_entity,
            entity,
            desc,
            AttributeDetailLevel::Basic,
            world,
        )
    }

    /// Creates an entity description for `entity`, with attribute descriptions of the provided detail level.
    fn for_entity_with_detail_level(
        pov_entity: Entity,
        entity: Entity,
        desc: &Description,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> EntityDescription {
        let pronouns = if entity == pov_entity {
            Pronouns::you()
        } else {
            desc.pronouns.clone()
        };

        EntityDescription {
            name: desc.name.clone(),
            aliases: build_aliases(desc),
            article: desc.article.clone(),
            pronouns,
            description: desc.description.clone(),
            attributes: desc
                .attribute_describers
                .iter()
                .flat_map(|d| d.describe(pov_entity, entity, detail_level, world))
                .collect(),
        }
    }
}

fn build_aliases(desc: &Description) -> Vec<String> {
    once(desc.room_name.clone())
        .chain(desc.aliases.clone())
        .filter(|name| name != &desc.name)
        .collect()
}

/// The detailed description of an entity.
#[derive(Debug, Clone)]
pub struct DetailedEntityDescription {
    pub basic_desc: EntityDescription,
    /// Descriptions of the actions that can be performed on the entity.
    pub actions: Vec<ActionDescription>,
}

impl DetailedEntityDescription {
    /// Creates a detailed entity description for `entity` being looked at by `looking_entity`.
    pub fn for_entity(
        looking_entity: Entity,
        entity: Entity,
        desc: &Description,
        world: &World,
    ) -> DetailedEntityDescription {
        DetailedEntityDescription {
            basic_desc: EntityDescription::for_entity_with_detail_level(
                looking_entity,
                entity,
                desc,
                AttributeDetailLevel::Advanced,
                world,
            ),
            actions: build_action_descriptions_for_entity(looking_entity, entity, world),
        }
    }
}

/// Builds a list of descriptions of actions `looking_entity` can perform on `entity`.
fn build_action_descriptions_for_entity(
    looking_entity: Entity,
    entity: Entity,
    world: &World,
) -> Vec<ActionDescription> {
    find_parsers_relevant_for(looking_entity, world)
        .flat_map(|p| p.get_input_formats_for(entity, looking_entity, world))
        .flatten()
        .unique()
        .map(|format| ActionDescription { format })
        .collect()
}
