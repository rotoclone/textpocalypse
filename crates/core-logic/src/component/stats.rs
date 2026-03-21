use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use bevy_ecs::prelude::*;
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    resource::{get_attribute_name, get_base_attribute, get_skill_name},
    send_message, GameMessage, IntegerExtensions, Notification, NotificationType,
};

/// The amount of XP needed for an entity to earn their first skill point.
const XP_FOR_FIRST_SKILL_POINT: Xp = Xp(100);
/// The amount of XP needed for an entity to earn their first attribute point.
const XP_FOR_FIRST_ATTRIBUTE_POINT: Xp = Xp(500);

/// How much more XP each advancement point needs than the previous one.
const ADVANCEMENT_POINT_NEXT_LEVEL_MULTIPLIER: f32 = 1.15;

/// The stats an entity started with, before spending any advancement points.
#[derive(Component)]
pub struct StartingStats(#[expect(unused)] pub Stats);

/// A unique key used to identify a set of stat modifications
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatModificationKey(&'static str);

/// The stats of an entity.
#[derive(Component, Clone)]
pub struct Stats {
    /// The innate attributes of the entity, like strength.
    pub attributes: Attributes,
    /// The learned skills of the entity, like cooking.
    pub skills: Skills,
    /// The entity's XP and stuff.
    pub advancement: StatAdvancement,
    /// The entity's active stat modifications
    modifications: HashMap<StatModificationKey, StatModifications>,
}

impl Stats {
    /// Creates a new set of stats with attributes and skills set to the provided default values.
    pub fn new(default_attribute_value: u16, default_skill_value: u16) -> Stats {
        Stats {
            attributes: Attributes::new(default_attribute_value),
            skills: Skills::new(default_skill_value),
            advancement: StatAdvancement::new(),
            modifications: HashMap::new(),
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

    /// Adds or updates the modifications with the provided key.
    pub fn set_modification(&mut self, key: StatModificationKey, modifications: StatModifications) {
        self.modifications.insert(key, modifications);
    }

    /// Removes the modifications with the provided key.
    pub fn remove_modification(&mut self, key: StatModificationKey) {
        self.modifications.remove(&key);
    }

    /// Gets all the active modifications for the provided stat.
    fn get_modifications(&self, stat: Stat) -> Vec<StatModification> {
        self.modifications
            .values()
            .flat_map(|modifications| modifications.0.get(&stat))
            .flatten()
            .copied()
            .collect()
    }

    /// Gets the total value of a skill, taking its base attribute and any active modifications into account.
    pub fn get_skill_total(&self, skill: &Skill, world: &World) -> f32 {
        let base_skill_value = self.skills.get_base(skill);
        let attribute_bonus = self.get_attribute_bonus(skill, world);
        let modifications = self.get_modifications(Stat::Skill(skill.clone()));

        let unmodified = f32::from(base_skill_value) + attribute_bonus;
        modifications.apply_to(unmodified)
    }

    /// Gets the total value of an attribute, take any active modifications into account.
    pub fn get_attribute_total(&self, attribute: &Attribute) -> f32 {
        todo!() //TODO
    }

    /// Determines the bonus to apply to the provided skill based on the value of its base attribute.
    pub fn get_attribute_bonus(&self, skill: &Skill, world: &World) -> f32 {
        let attribute = get_base_attribute(skill, world);
        let attribute_value = self.get_attribute_total(&attribute);

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

    /// Gets the base value of the provided attribute.
    pub fn get_base(&self, attribute: &Attribute) -> u16 {
        *match attribute {
            Attribute::Custom(s) => self.custom.get(s),
            a => self.standard.get(a),
        }
        .unwrap_or(&0)
    }

    /// Gets all the attributes and their base values.
    pub fn get_all_base(&self) -> Vec<(Attribute, u16)> {
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
    /// TODO rename this and related functions to `get_raw`, since there's also the concept of a "base attribute" for a skill
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

/* TODO remove
/// A notification used to collect active stat modifications for an entity.
/// TODO send this when getting stat values
/// TODO is it silly to gather this information via notifications rather than just keeping track of the stat modifications on the stats component itself?
#[derive(Debug)]
pub struct GetStatModificationsNotification {
    /// The entity to get stat modifications for
    pub entity: Entity,
    /// The stat to get modifications for
    pub stat: Stat,
}

impl NotificationType for GetStatModificationsNotification {
    type Return = Vec<StatModification>;
}
    */

/// A set of modifications to various attributes and/or skills.
#[derive(Clone)]
pub struct StatModifications(HashMap<Stat, Vec<StatModification>>);

/// A modification to a single stat.
#[derive(Clone, Copy)]
pub enum StatModification {
    /// Increase the stat's value
    Add(f32),
    /// Decrease the stat's value
    Subtract(f32),
    /// Multiply the stat's value
    Multiply(f32),
}

impl StatModification {
    /// Gets a value used to sort modifications by type.
    fn get_compare_key(&self) -> i8 {
        match self {
            StatModification::Add(_) => 0,
            StatModification::Subtract(_) => 1,
            StatModification::Multiply(_) => 2,
        }
    }

    /// Applies the modification to the provided value.
    fn apply(&self, value: f32) -> f32 {
        match self {
            StatModification::Add(x) => value + x,
            StatModification::Subtract(x) => value - x,
            StatModification::Multiply(x) => value * x,
        }
    }
}

trait ApplyStatModificationsTo {
    /// Applies the modifications to the provided stat value.
    /// Will return zero if the modified value would be negative.
    fn apply_to(&self, stat_value: f32) -> f32;
}

impl ApplyStatModificationsTo for Vec<StatModification> {
    fn apply_to(&self, stat_value: f32) -> f32 {
        let mut modified = stat_value;
        self.iter()
            .sorted_by(|a, b| a.get_compare_key().cmp(&b.get_compare_key()))
            .for_each(|modification| modified = modification.apply(modified));

        modified.max(0.0)
    }
}

impl StatModifications {
    /// Creates a new empty set of modifications.
    pub fn new() -> StatModifications {
        StatModifications(HashMap::new())
    }

    /// Adds a modification for a stat.
    /// Any previously-added modifications for the same attribute will not be replaced.
    pub fn modify_stat(mut self, stat: Stat, modification: StatModification) -> StatModifications {
        self.0.entry(stat).or_default().push(modification);

        self
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

impl NotificationType for XpAwardNotification {
    type Return = ();
}

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
    /// TODO should this instead get values in bulk, to reduce the number of notifications sent to get modifications?
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
