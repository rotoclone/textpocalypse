use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::{Attributes, Stats},
    resource::{get_attribute_name, get_base_attribute, get_skill_name},
};

/// The description of an entity's stats.
#[derive(Debug, Clone)]
pub struct StatsDescription {
    /// The attributes of the entity.
    pub attributes: Vec<StatAttributeDescription>,
    /// The skills of the entity.
    pub skills: Vec<SkillDescription>,
}

impl StatsDescription {
    pub fn from_stats(stats: &Stats, world: &World) -> StatsDescription {
        StatsDescription {
            attributes: StatAttributeDescription::from_attributes(&stats.attributes, world),
            skills: SkillDescription::from_stats(stats, world),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatAttributeDescription {
    pub name: String,
    pub value: u16,
}

impl StatAttributeDescription {
    fn from_attributes(attributes: &Attributes, world: &World) -> Vec<StatAttributeDescription> {
        attributes
            .get_all()
            .into_iter()
            .map(|(attribute, value)| StatAttributeDescription {
                name: get_attribute_name(&attribute, world).full,
                value,
            })
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SkillDescription {
    pub name: String,
    pub base_attribute_name: String,
    pub attribute_bonus: f32,
    pub base_value: u16,
    pub total: f32,
}

impl SkillDescription {
    fn from_stats(stats: &Stats, world: &World) -> Vec<SkillDescription> {
        stats
            .skills
            .get_all_base()
            .into_iter()
            .map(|(skill, base_value)| {
                let base_attribute = get_base_attribute(&skill, world);
                let attribute_bonus = stats.get_attribute_bonus(&skill, world);
                SkillDescription {
                    name: get_skill_name(&skill, world),
                    base_attribute_name: get_attribute_name(&base_attribute, world).short,
                    attribute_bonus,
                    base_value,
                    total: stats.get_skill_total(&skill, world),
                }
            })
            .sorted_by(|a, b| {
                // group skills by base attribute, then alphabetically
                if a.base_attribute_name == b.base_attribute_name {
                    a.name.cmp(&b.name)
                } else {
                    a.base_attribute_name.cmp(&b.base_attribute_name)
                }
            })
            .collect()
    }
}
