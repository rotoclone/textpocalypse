use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use bevy_ecs::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    resource::{get_attribute_name, get_base_attribute, get_skill_name},
    send_message, GameMessage, IntegerExtensions, Notification, NotificationType,
};

/// The amount of XP needed for an entity to earn their first skill point.
const XP_FOR_FIRST_SKILL_POINT: Xp = Xp(100);
/// The amount of XP needed for an entity to earn their first attribute point.
const XP_FOR_FIRST_ATTRIBUTE_POINT: Xp = Xp(300);

/// How much more XP each advancement point needs than the previous one.
const ADVANCEMENT_POINT_NEXT_LEVEL_MULTIPLIER: f32 = 1.2;

/// The stats an entity started with, before spending any advancement points.
#[derive(Component)]
pub struct StartingStats(pub Stats);

/// The stats of an entity.
#[derive(Component, Clone)]
pub struct Stats {
    /// The innate attributes of the entity, like strength.
    pub attributes: Attributes,
    /// The learned skills of the entity, like cooking.
    pub skills: Skills,
    /// The entity's XP and stuff.
    pub advancement: StatAdvancement,
}

impl Stats {
    /// Creates a new set of stats with attributes and skills set to the provided default values.
    pub fn new(default_attribute_value: u16, default_skill_value: u16) -> Stats {
        Stats {
            attributes: Attributes::new(default_attribute_value),
            skills: Skills::new(default_skill_value),
            advancement: StatAdvancement::new(),
        }
    }

    /// Sets an attribute to a specific value.
    pub fn set_attribute(&mut self, attribute: &Attribute, value: u16) {
        match attribute {
            Attribute::Custom(s) => self.attributes.custom.insert(s.clone(), value),
            a => self.attributes.standard.insert(a.clone(), value),
        };
    }

    /// Sets a skill to a specific value.
    pub fn set_skill(&mut self, skill: &Skill, value: u16) {
        match skill {
            Skill::Custom(s) => self.skills.custom.insert(s.clone(), value),
            a => self.skills.standard.insert(a.clone(), value),
        };
    }

    /// Gets the total value of a skill, taking its base attribute into account.
    pub fn get_skill_total(&self, skill: &Skill, world: &World) -> f32 {
        let base_skill_value = self.skills.get_base(skill);
        let attribute_bonus = self.get_attribute_bonus(skill, world);

        f32::from(base_skill_value) + attribute_bonus
    }

    /// Determines the bonus to apply to the provided skill based on the value of its base attribute.
    pub fn get_attribute_bonus(&self, skill: &Skill, world: &World) -> f32 {
        let attribute = get_base_attribute(skill, world);
        let attribute_value = self.attributes.get(&attribute);

        f32::from(attribute_value) / 2.0
    }
}

/// The innate attributes of an entity, like strength.
#[derive(Clone)]
pub struct Attributes {
    standard: HashMap<Attribute, u16>,
    custom: HashMap<String, u16>,
}

impl Attributes {
    fn new(default_value: u16) -> Attributes {
        let mut standard = HashMap::new();
        for attribute in Attribute::iter() {
            match attribute {
                Attribute::Custom(_) => (),
                a => {
                    standard.insert(a, default_value);
                }
            }
        }

        Attributes {
            standard,
            custom: HashMap::new(),
        }
    }

    /// Gets the value of the provided attribute.
    pub fn get(&self, attribute: &Attribute) -> u16 {
        *match attribute {
            Attribute::Custom(s) => self.custom.get(s),
            a => self.standard.get(a),
        }
        .unwrap_or(&0)
    }

    /// Gets all the attributes and their values.
    pub fn get_all(&self) -> Vec<(Attribute, u16)> {
        let standards = self
            .standard
            .iter()
            .map(|entry| (entry.0.clone(), *entry.1));

        let customs = self
            .custom
            .iter()
            .map(|entry| (Attribute::Custom(entry.0.clone()), *entry.1));

        standards.chain(customs).collect()
    }
}

/// The learned skills of an entity, like cooking.
#[derive(Clone)]
pub struct Skills {
    standard: HashMap<Skill, u16>,
    custom: HashMap<String, u16>,
}

impl Skills {
    fn new(default_value: u16) -> Skills {
        let mut standard = HashMap::new();
        for skill in Skill::iter() {
            match skill {
                Skill::Custom(_) => (),
                s => {
                    standard.insert(s, default_value);
                }
            }
        }

        Skills {
            standard,
            custom: HashMap::new(),
        }
    }

    /// Gets the base value of the provided skill.
    pub fn get_base(&self, skill: &Skill) -> u16 {
        *match skill {
            Skill::Custom(s) => self.custom.get(s),
            a => self.standard.get(a),
        }
        .unwrap_or(&0)
    }

    /// Gets all the skills and their base values.
    pub fn get_all_base(&self) -> Vec<(Skill, u16)> {
        let standards = self
            .standard
            .iter()
            .map(|entry| (entry.0.clone(), *entry.1));

        let customs = self
            .custom
            .iter()
            .map(|entry| (Skill::Custom(entry.0.clone()), *entry.1));

        standards.chain(customs).collect()
    }
}

/// An amount of experience points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Xp(pub u64);

/// A notification that an entity is getting some XP.
///
/// Generally, XP should only be awarded for "risky" actions, to avoid creating easy XP-grinding sources.
/// If an action can be done repeatedly without any risk or using up any limited resources, it probably shouldn't award XP.
#[derive(Debug)]
pub struct XpAwardNotification {
    /// The entity getting XP
    pub entity: Entity,
    /// The XP the entity is getting
    pub xp_to_add: Xp,
}

impl NotificationType for XpAwardNotification {}

/// Increases an entity's XP total when they are given XP, and awards any advancement points that are warranted.
pub fn increase_xp_and_advancement_points_on_xp_awarded(
    notification: &Notification<XpAwardNotification, ()>,
    world: &mut World,
) {
    let entity = notification.notification_type.entity;
    if let Some(mut stats) = world.get_mut::<Stats>(entity) {
        stats.advancement.total_xp.0 += notification.notification_type.xp_to_add.0;

        let mut messages = Vec::new();

        let mut skill_points_gained = 0;
        while stats.advancement.skill_points.xp_for_next <= stats.advancement.total_xp {
            stats.advancement.skill_points.award_one();
            skill_points_gained += 1;
        }
        if skill_points_gained > 0 {
            messages.push(GameMessage::AdvancementPointsGained(
                skill_points_gained,
                AdvancementPointType::Skill,
            ));
        }

        let mut attribute_points_gained = 0;
        while stats.advancement.attribute_points.xp_for_next <= stats.advancement.total_xp {
            stats.advancement.attribute_points.award_one();
            attribute_points_gained += 1;
        }
        if attribute_points_gained > 0 {
            messages.push(GameMessage::AdvancementPointsGained(
                attribute_points_gained,
                AdvancementPointType::Attribute,
            ));
        }

        for message in messages {
            send_message(world, entity, message);
        }
    }
}

/// Information about an entity's available avenues of increasing their stats.
#[derive(Clone)]
pub struct StatAdvancement {
    /// The total amount of XP the entity has earned
    pub total_xp: Xp,
    /// The skill points of the entity
    pub skill_points: AdvancementPoints,
    /// The attribute points of the entity
    pub attribute_points: AdvancementPoints,
}

/// The skill or attribute points of an entity.
#[derive(Clone)]
pub struct AdvancementPoints {
    /// The total number of points the entity has earned
    pub total_earned: u32,
    /// The number of unspent points
    pub available: u32,
    /// The amount of XP needed for the next point
    pub xp_for_next: Xp,
}

/// A type of advancement point.
#[derive(Debug, Clone)]
pub enum AdvancementPointType {
    /// A skill point
    Skill,
    /// An attribute point
    Attribute,
}

impl AdvancementPoints {
    fn new(xp_for_first: Xp) -> AdvancementPoints {
        AdvancementPoints {
            total_earned: 0,
            available: 0,
            xp_for_next: xp_for_first,
        }
    }

    /// Adds one advancement point, and updates the XP needed for the next one.
    fn award_one(&mut self) {
        self.total_earned += 1;
        self.available += 1;
        self.xp_for_next.0 += self
            .xp_for_next
            .0
            .mul_and_round(ADVANCEMENT_POINT_NEXT_LEVEL_MULTIPLIER);
    }
}

impl StatAdvancement {
    fn new() -> StatAdvancement {
        StatAdvancement {
            total_xp: Xp(0),
            skill_points: AdvancementPoints::new(XP_FOR_FIRST_SKILL_POINT),
            attribute_points: AdvancementPoints::new(XP_FOR_FIRST_ATTRIBUTE_POINT),
        }
    }
}

/// A stat (i.e. either an attribute or a skill)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stat {
    Attribute(Attribute),
    Skill(Skill),
}

impl From<Skill> for Stat {
    fn from(value: Skill) -> Self {
        Stat::Skill(value)
    }
}

impl From<Attribute> for Stat {
    fn from(value: Attribute) -> Self {
        Stat::Attribute(value)
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stat::Attribute(attribute) => attribute.fmt(f),
            Stat::Skill(skill) => skill.fmt(f),
        }
    }
}

impl Stat {
    /// Gets the value of this stat.
    pub fn get_value(&self, stats: &Stats, world: &World) -> f32 {
        match self {
            Stat::Attribute(attribute) => f32::from(stats.attributes.get(attribute)),
            Stat::Skill(skill) => stats.get_skill_total(skill, world),
        }
    }

    /// Gets the provided entity's value for this stat, if they have stats.
    pub fn get_entity_value(&self, entity: Entity, world: &World) -> Option<f32> {
        world
            .get::<Stats>(entity)
            .map(|stats| self.get_value(stats, world))
    }

    /// Gets the display name of this stat.
    pub fn get_name(&self, world: &World) -> String {
        match self {
            Stat::Attribute(attribute) => get_attribute_name(attribute, world).full,
            Stat::Skill(skill) => get_skill_name(skill, world),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Attribute {
    Strength,
    Agility,
    Intelligence,
    Perception,
    Endurance,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
pub enum Skill {
    Firearms,
    Bows,
    Blades,
    Bludgeons,
    Fists,
    Construction,
    Craft,
    Scavenge,
    Stealth,
    Medicine,
    Cook,
    Dodge,
    Climb,
    Lockpick,
    Butchery,
    Custom(String),
}
