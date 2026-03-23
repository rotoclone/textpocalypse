use bevy_ecs::prelude::*;
use itertools::Itertools;

use crate::{
    component::Stats,
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
            attributes: StatAttributeDescription::from_stats(stats, world),
            skills: SkillDescription::from_stats(stats, world),
            advancement: AdvancementDescription::from_advancement(&stats.advancement),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatAttributeDescription {
    /// The name of the attribute
    pub name: String,
    /// The raw value of the attribute
    pub raw_value: u16,
    /// The total value of active modifications to the attribute
    pub modifications: f32,
    /// The total value of the attribute, after any active modifications are applied
    pub total: f32,
}

impl StatAttributeDescription {
    fn from_stats(stats: &Stats, world: &World) -> Vec<StatAttributeDescription> {
        stats
            .attributes
            .get_all()
            .into_iter()
            .map(|attribute| {
                let attribute_value = stats.get_attribute_value(&attribute);
                StatAttributeDescription {
                    name: get_attribute_name(&attribute, world).full,
                    raw_value: attribute_value.raw,
                    modifications: attribute_value.modifications,
                    total: attribute_value.total,
                }
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
    /// The raw value of the skill
    pub raw_value: u16,
    /// The bonus the base attribute confers to the skill's value
    pub attribute_bonus: f32,
    /// The total value of active modifications to the skill
    pub modifications: f32,
    /// The total value of the skill, after the attribute bonus and any active modifications are applied
    pub total: f32,
}

impl SkillDescription {
    fn from_stats(stats: &Stats, world: &World) -> Vec<SkillDescription> {
        stats
            .skills
            .get_all()
            .into_iter()
            .map(|skill| {
                let base_attribute = get_base_attribute(&skill, world);
                let skill_value = stats.get_skill_value(&skill, world);
                SkillDescription {
                    name: get_skill_name(&skill, world),
                    base_attribute_name: get_attribute_name(&base_attribute, world).short,
                    raw_value: skill_value.raw,
                    attribute_bonus: skill_value.attribute_bonus,
                    modifications: skill_value.modifications,
                    total: skill_value.total,
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
