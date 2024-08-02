use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::{Attributes, Stats},
    resource::{get_attribute_name, get_base_attribute, get_skill_name},
    AdvancementPoints, StatAdvancement, Xp,
};

/// The description of an entity's stats.
#[derive(Debug, Clone)]
pub struct StatsDescription {
    /// The attributes of the entity.
    pub attributes: Vec<StatAttributeDescription>,
    /// The skills of the entity.
    pub skills: Vec<SkillDescription>,
    /// The entity's XP and stuff.
    pub advancement: AdvancementDescription,
}

impl StatsDescription {
    pub fn from_stats(stats: &Stats, world: &World) -> StatsDescription {
        StatsDescription {
            attributes: StatAttributeDescription::from_attributes(&stats.attributes, world),
            skills: SkillDescription::from_stats(stats, world),
            advancement: AdvancementDescription::from_advancement(&stats.advancement),
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
    /// The name of the skill
    pub name: String,
    /// The name of the base attribute for the skill
    pub base_attribute_name: String,
    /// The bonus the base attribute confers to the skill's value
    pub attribute_bonus: f32,
    /// The base value of the skill
    pub base_value: u16,
    /// The total value of the skill
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

#[derive(Debug, Clone)]
pub struct AdvancementDescription {
    /// The total amount of XP the entity has earned
    pub total_xp: Xp,
    /// The skill points of the entity
    pub skill_points: AdvancementPointsDescription,
    /// The attribute points of the entity
    pub attribute_points: AdvancementPointsDescription,
}

#[derive(Debug, Clone)]
pub struct AdvancementPointsDescription {
    /// The number of points available to spend
    pub available: u32,
    /// The amount of XP needed for the next point
    pub xp_for_next: Xp,
}

impl AdvancementDescription {
    fn from_advancement(advancement: &StatAdvancement) -> AdvancementDescription {
        AdvancementDescription {
            total_xp: advancement.total_xp,
            skill_points: AdvancementPointsDescription::from_advancement_points(
                &advancement.skill_points,
            ),
            attribute_points: AdvancementPointsDescription::from_advancement_points(
                &advancement.attribute_points,
            ),
        }
    }
}

impl AdvancementPointsDescription {
    fn from_advancement_points(
        advancement_points: &AdvancementPoints,
    ) -> AdvancementPointsDescription {
        AdvancementPointsDescription {
            available: advancement_points.available,
            xp_for_next: advancement_points.xp_for_next,
        }
    }
}
