use std::collections::HashSet;

use bevy_ecs::prelude::*;
use log::debug;
use ordered_float::OrderedFloat;
use voca_rs::Voca;

use crate::{component::Room, find_owning_entity, is_living_entity, GameMessage};

use super::Location;

/// The description of an entity.
#[derive(Component, Debug)]
pub struct Description {
    /// The name of the entity.
    pub name: String,
    /// The name to use when referring to the entity as part of a room description.
    pub room_name: String,
    /// The name to use when referring to multiple instances of the entity.
    pub plural_name: String,
    /// The article to use when referring to the entity (usually "a" or "an").
    pub article: Option<String>,
    /// The pronouns to use when referring to the entity.
    pub pronouns: Pronouns,
    /// The alternate names of the entity.
    pub aliases: Vec<String>,
    /// The description of the entity.
    pub description: String,
    /// Describers for dynamic attributes of the entity.
    pub attribute_describers: Vec<Box<dyn AttributeDescriber>>,
}

#[derive(Debug, Clone)]
pub struct Pronouns {
    /// The personal subject form (e.g. he, she, they)
    pub personal_subject: String,
    /// The personal object form (e.g. him, her, them)
    pub personal_object: String,
    /// The possessive form (e.g. his, hers, theirs)
    pub possessive: String,
    /// The possessive adjective form (e.g. his, her, their).
    pub possessive_adjective: String,
    /// The reflexive form (e.g. himself, herself, themself)
    pub reflexive: String,
    /// Whether the pronouns are considered to be plural or not.
    pub plural: bool,
}

impl Pronouns {
    /// Creates a set of pronouns.
    pub fn new<T: Into<String>>(
        personal_subject: T,
        personal_object: T,
        possessive: T,
        possessive_adjective: T,
        reflexive: T,
        plural: bool,
    ) -> Pronouns {
        Pronouns {
            personal_subject: personal_subject.into(),
            personal_object: personal_object.into(),
            possessive: possessive.into(),
            possessive_adjective: possessive_adjective.into(),
            reflexive: reflexive.into(),
            plural,
        }
    }

    /// Creates a set of pronouns with forms of "he".
    pub fn he() -> Pronouns {
        Pronouns::new("he", "him", "his", "his", "himself", false)
    }

    /// Creates a set of pronouns with forms of "she".
    pub fn she() -> Pronouns {
        Pronouns::new("she", "her", "hers", "her", "herself", false)
    }

    /// Creates a set of pronouns with forms of "they".
    pub fn they() -> Pronouns {
        Pronouns::new("they", "them", "theirs", "their", "themself", true)
    }

    /// Creates a set of pronouns with forms of "you".
    pub fn you() -> Pronouns {
        Pronouns::new("you", "you", "yours", "your", "yourself", true)
    }

    /// Creates a set of pronouns with forms of "it".
    pub fn it() -> Pronouns {
        Pronouns::new("it", "it", "its", "its", "itself", false)
    }

    /// Gets the personal subject pronoun to use when referring to the provided entity (e.g. he, she, they).
    ///
    /// If a POV entity is provided, and it's the same as the entity, this will return "you".
    /// If the entity has no description and is alive, this will return "they".
    /// If the entity has no description and is not alive, this will return "it".
    pub fn get_personal_subject(
        entity: Entity,
        pov_entity: Option<Entity>,
        world: &World,
    ) -> String {
        if pov_entity == Some(entity) {
            "you".to_string()
        } else if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.personal_subject.clone()
        } else if is_living_entity(entity, world) {
            "they".to_string()
        } else {
            "it".to_string()
        }
    }

    /// Gets the personal object pronoun to use when referring to the provided entity (e.g. him, her, them).
    ///
    /// If a POV entity is provided, and it's the same as the entity, this will return "you".
    /// If the entity has no description and is alive, this will return "them".
    /// If the entity has no description and is not alive, this will return "it".
    pub fn get_personal_object(
        entity: Entity,
        pov_entity: Option<Entity>,
        world: &World,
    ) -> String {
        if pov_entity == Some(entity) {
            "you".to_string()
        } else if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.personal_object.clone()
        } else if is_living_entity(entity, world) {
            "them".to_string()
        } else {
            "it".to_string()
        }
    }

    /// Gets the possessive pronoun to use when referring to the provided entity (e.g. his, hers, theirs).
    ///
    /// If a POV entity is provided, and it's the same as the entity, this will return "yours".
    /// If the entity has no description and is alive, this will return "theirs".
    /// If the entity has no description and is not alive, this will return "its".
    pub fn get_possessive(entity: Entity, pov_entity: Option<Entity>, world: &World) -> String {
        if pov_entity == Some(entity) {
            "yours".to_string()
        } else if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.possessive.clone()
        } else if is_living_entity(entity, world) {
            "theirs".to_string()
        } else {
            "its".to_string()
        }
    }

    /// Gets the possessive adjective pronoun to use when referring to the provided entity (e.g. his, her, their).
    ///
    /// If a POV entity is provided, and it's the same as the entity, this will return "your".
    /// If the entity has no description and is alive, this will return "their".
    /// If the entity has no description and is not alive, this will return "its".
    pub fn get_possessive_adjective(
        entity: Entity,
        pov_entity: Option<Entity>,
        world: &World,
    ) -> String {
        if pov_entity == Some(entity) {
            "your".to_string()
        } else if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.possessive_adjective.clone()
        } else if is_living_entity(entity, world) {
            "their".to_string()
        } else {
            "its".to_string()
        }
    }

    /// Gets the reflexive pronoun to use when referring to the provided entity (e.g. himself, herself, themself).
    ///
    /// If a POV entity is provided, and it's the same as the entity, this will return "yourself".
    /// If the entity has no description and is alive, this will return "themself".
    /// If the entity has no description and is not alive, this will return "itself".
    pub fn get_reflexive(entity: Entity, pov_entity: Option<Entity>, world: &World) -> String {
        if pov_entity == Some(entity) {
            "yourself".to_string()
        } else if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.reflexive.clone()
        } else if is_living_entity(entity, world) {
            "themself".to_string()
        } else {
            "itself".to_string()
        }
    }

    /// Gets the form of "to be" to use when referring to the provided entity (i.e. is/are).
    ///
    /// If the entity has no description and is alive, this will return "are" (to be paired with "they").
    /// If the entity has no description and is not alive, this will return "is" (to be paired with "it").
    pub fn get_to_be_form(entity: Entity, world: &World) -> String {
        if let Some(desc) = world.get::<Description>(entity) {
            if desc.pronouns.plural {
                "are".to_string()
            } else {
                "is".to_string()
            }
        } else if is_living_entity(entity, world) {
            "are".to_string()
        } else {
            "is".to_string()
        }
    }

    /// Determines whether the provided entity's pronouns are plural.
    ///
    /// If the entity has no description and is alive, this will return true (to be paired with "they").
    /// If the entity has no description and is not alive, this will return false (to be paired with "it").
    pub fn is_plural(entity: Entity, world: &World) -> bool {
        if let Some(desc) = world.get::<Description>(entity) {
            desc.pronouns.plural
        } else {
            is_living_entity(entity, world)
        }
    }
}

/// Describes how closely a given input matches with an entity's description.
pub enum Matchness {
    /// The input matches one of the entity's identifiers exactly
    Exact,
    /// The input partially matches one of the entity's identifiers (such as "pan" and the name "pants")
    Partial(PortionMatched),
    /// The input doesn't match at all with any of the entity's identifiers
    None,
}

/// Contains a float between 0 and 1 (exclusive) representing the fraction of a name that matched with a given input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortionMatched(pub OrderedFloat<f32>);

impl Description {
    /// Determines how closely the provided input refers to the entity with this description.
    pub fn matches(&self, input: &str) -> Matchness {
        let mut identifiers = Vec::new();
        identifiers.push(self.name.to_ascii_lowercase());
        identifiers.push(self.room_name.to_ascii_lowercase());
        identifiers.extend(self.aliases.iter().map(|alias| alias.to_ascii_lowercase()));

        // allow optionally prefixing with "the"
        let identifiers_with_the = identifiers
            .iter()
            .map(|identifier| format!("the {identifier}"))
            .collect::<Vec<String>>();
        identifiers.extend(identifiers_with_the);

        let mut best_portion_matched = None;
        for identifier in identifiers {
            let identifier_length = identifier._count_graphemes();
            if input._count_graphemes() > identifier_length {
                // if the input is longer than this identifier then it can't match
                continue;
            }
            let unmatched_part = identifier._removeprefix(&input.to_ascii_lowercase());
            let graphemes_matched = identifier_length - unmatched_part._count_graphemes();

            if graphemes_matched == identifier_length {
                return Matchness::Exact;
            }
            if graphemes_matched > 0 {
                let portion_matched = PortionMatched(OrderedFloat(
                    graphemes_matched as f32 / identifier_length as f32,
                ));
                if portion_matched
                    > best_portion_matched.unwrap_or(PortionMatched(OrderedFloat(0.0)))
                {
                    best_portion_matched = Some(portion_matched);
                }
            }
        }

        if let Some(portion_matched) = best_portion_matched {
            return Matchness::Partial(portion_matched);
        }

        Matchness::None
    }

    /// Gets the name of the provided entity, if it has one.
    ///
    /// If the entity has no Description, but is a Room, this will return the name of the room.
    pub fn get_name(entity: Entity, world: &World) -> Option<String> {
        world
            .get::<Description>(entity)
            .map(|d| d.name.clone())
            .or_else(|| world.get::<Room>(entity).map(|r| r.name.clone()))
    }

    /// Builds a string to use to refer to the provided entity from the point of view of another entity.
    ///
    /// For example, if the entity is named "book", this will return "the book".
    ///
    /// If `pov_entity` is the same as `entity`, this will return "you".
    pub fn get_reference_name(entity: Entity, pov_entity: Option<Entity>, world: &World) -> String {
        if Some(entity) == pov_entity {
            return "you".to_string();
        }

        let article = Description::get_definite_article(entity, pov_entity, world)
            .map_or_else(|| "".to_string(), |a| format!("{a} "));
        Description::get_name(entity, world)
            .map_or("it".to_string(), |name| format!("{article}{name}"))
    }

    /// Gets the definite article to use when referring to the provided entity.
    ///
    /// If `pov_entity` owns it, this will return `Some("your")`.
    ///
    /// If some other entity owns it, this will return that entity's possessive adjective pronoun (e.g. "his", "her", "their", etc.).
    ///
    /// Otherwise, this will return `Some("the")` if the entity has no description or has an article defined in its description,
    /// or `None` if the entity has a description but no article.
    pub fn get_definite_article(
        entity: Entity,
        pov_entity: Option<Entity>,
        world: &World,
    ) -> Option<String> {
        let owning_entity = find_owning_entity(entity, world);

        if let Some(owning_entity) = owning_entity {
            return Some(Pronouns::get_possessive_adjective(
                owning_entity,
                pov_entity,
                world,
            ));
        }

        if world
            .get::<Description>(entity)
            .map(|d| d.article.is_none())
            .unwrap_or(false)
        {
            return None;
        }
        Some("the".to_string())
    }

    /// Builds a string to use to refer to the provided entity generically.
    ///
    /// For example, if the entity is named "book" and has its article set to "a", this will return "a book".
    pub fn get_article_reference_name(entity: Entity, world: &World) -> String {
        if let Some(desc) = world.get::<Description>(entity) {
            if let Some(article) = &desc.article {
                format!("{} {}", article, desc.name)
            } else {
                desc.name.clone()
            }
        } else if is_living_entity(entity, world) {
            "someone".to_string()
        } else {
            "something".to_string()
        }
    }

    /// Finds all the strings representing ways to reference `entity`` from the perspective of `pov_entity`.
    /// TODO this should probably either be removed or adapted to be able to be used in `matches` above
    pub fn get_all_ways_to_reference(
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> HashSet<&str> {
        let mut names = HashSet::new();
        if let Some(desc) = world.get::<Description>(entity) {
            names.insert(desc.name.as_str());
            names.insert(desc.room_name.as_str());
            names.extend(desc.aliases.iter().map(|a| a.as_str()))
        }

        if entity == pov_entity {
            names.insert("me");
        }

        if let Some(location) = world.get::<Location>(pov_entity) {
            if entity == location.id {
                names.insert("here");
            }
        }

        names
    }
}

pub trait AttributeDescriber: Send + Sync + std::fmt::Debug {
    /// Generates descriptions of attributes an entity from the perspective of another entity.
    fn describe(
        &self,
        pov_entity: Entity,
        entity: Entity,
        detail_level: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription>;
}

/// The level of detail to use for attribute descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeDetailLevel {
    /// Basic details, like whether the entity is open.
    Basic = 0,
    /// Advanced details, like how much the entity weighs.
    Advanced,
}

/// A description of a single attribute of an entity.
#[derive(Debug, Clone)]
pub enum AttributeDescription {
    /// An attribute that should be described as a regular sentence, like the fact that an entity is closed.
    NonSection(NonSectionAttributeDescription),
    /// An attribute or set of attributes that should be described in a formatted section, like weapon stats.
    Section(AttributeSection),
    /// A description in the form of a game message, like the contents of an entity.
    /// Boxed to reduce overall size of the enum, since this variant should be comparatively rare.
    Message(Box<GameMessage>),
}

impl AttributeDescription {
    /// Creates a description of something an entity is, like "closed" or "broken".
    pub fn is(description: String) -> AttributeDescription {
        AttributeDescription::NonSection(NonSectionAttributeDescription {
            attribute_type: NonSectionAttributeType::Is,
            description,
        })
    }

    /// Creates a description of something an entity does, like "glows" or "makes you feel uneasy".
    pub fn does(description: String) -> AttributeDescription {
        AttributeDescription::NonSection(NonSectionAttributeDescription {
            attribute_type: NonSectionAttributeType::Does,
            description,
        })
    }

    /// Creates a description of something an entity has, like "3 uses left" or "some bites taken out of it".
    pub fn has(description: String) -> AttributeDescription {
        AttributeDescription::NonSection(NonSectionAttributeDescription {
            attribute_type: NonSectionAttributeType::Has,
            description,
        })
    }

    /// Creates a description of something an entity is wearing, like "pants".
    pub fn wears(description: String) -> AttributeDescription {
        AttributeDescription::NonSection(NonSectionAttributeDescription {
            attribute_type: NonSectionAttributeType::Wears,
            description,
        })
    }

    /// Creates a description of something an entity is wielding, like "a rock".
    pub fn wields(description: String) -> AttributeDescription {
        AttributeDescription::NonSection(NonSectionAttributeDescription {
            attribute_type: NonSectionAttributeType::Wields,
            description,
        })
    }
}

/// A description of a single attribute of an entity that shouldn't be part of a description section.
#[derive(Debug, Clone)]
pub struct NonSectionAttributeDescription {
    /// The type of attribute.
    pub attribute_type: NonSectionAttributeType,
    /// The descrption of the attribute.
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum NonSectionAttributeType {
    /// Something the entity is, like "closed" or "broken".
    Is,
    /// Something the entity does, like "glows" or "makes you feel uneasy".
    Does,
    /// Something the entity has, like "3 uses left" or "some bites taken out of it".
    Has,
    /// Something the entity is wearing, like "pants".
    Wears,
    /// Something the entity is wielding, like "a rock".
    Wields,
}

#[derive(Debug, Clone)]
pub struct AttributeSection {
    /// The name of the section
    pub name: AttributeSectionName,
    /// The attributes in the section
    pub attributes: Vec<SectionAttributeDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AttributeSectionName {
    Item,
    Edible,
    Wearable,
    Weapon,
    FluidContainer,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct SectionAttributeDescription {
    /// The name of the attribute, e.g. "Weight"
    pub name: String,
    /// The description of the attribute, e.g. "10 kg"
    pub description: String,
}

/// Trait for components that have describable attributes.
pub trait DescribeAttributes {
    /// Registers the attribute describer for this component on the provided entity.
    #[expect(unused)]
    fn register_attribute_describer(entity: Entity, world: &mut World) {
        if let Some(mut description) = world.get_mut::<Description>(entity) {
            description
                .attribute_describers
                .push(Self::get_attribute_describer());
        } else {
            world.entity_mut(entity).insert(Description {
                name: "".to_string(),
                room_name: "".to_string(),
                plural_name: "".to_string(),
                article: None,
                pronouns: Pronouns::it(),
                aliases: Vec::new(),
                description: "".to_string(),
                attribute_describers: vec![Self::get_attribute_describer()],
            });
        }
    }

    /// Returns the `AttributeDescriber` for this component.
    fn get_attribute_describer() -> Box<dyn AttributeDescriber>;
}
