use bevy_ecs::prelude::*;
use strum::EnumIter;

use crate::{
    action::PutAction,
    component::{
        description::NonSectionAttributeDescription, AttributeDescriber, AttributeDetailLevel,
        Container, DescribeAttributes, Description, FirearmMagazine, ParseCustomInput,
        SectionAttributeDescription, VerifyActionNotification, VerifyResult,
    },
    input_parser::InputParser,
    notification::{Notification, ReturningNotificationHandlers},
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    AttributeDescription, AttributeSection, AttributeSectionName, GameMessage,
    NonSectionAttributeType,
};

/// The caliber of ammunition a firearm accepts.
#[derive(PartialEq, Eq, Hash, Clone, EnumIter)]
pub enum AmmoCaliber {
    /// 9mm
    NineMm,
    /// A custom caliber
    Custom(String),
}

/// Component for entities that are firearms.
#[derive(Component)]
pub struct Firearm {
    /// The caliber of bullet this firearm can shoot
    pub caliber: AmmoCaliber,
}

impl ParseCustomInput for Firearm {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![] //TODO add reload action
    }
}

impl DescribeAttributes for Firearm {
    fn get_attribute_describer() -> Box<dyn AttributeDescriber> {
        Box::new(FirearmAttributeDescriber)
    }
}

impl Firearm {
    /// Registers handlers for gun actions.
    pub fn register_handlers(world: &mut World) {
        ReturningNotificationHandlers::add_handler(verify_item_to_put_in_firearm, world);
    }
}

/// Describes a firearm.
#[derive(Debug)]
struct FirearmAttributeDescriber;

impl AttributeDescriber for FirearmAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let Some(firearm) = world.get::<Firearm>(entity) else {
            return Vec::new();
        };

        let container = world
            .get::<Container>(entity)
            .expect("firearm should be a container");

        let contents = container.get_entities_including_invisible();

        let loaded_desc = if contents.is_empty() {
            "unloaded".to_string()
        } else if contents.len() == 1 {
            // unwrap is safe due to the length check above
            let contained = contents.iter().next().unwrap();
            let magazine = world
                .get::<FirearmMagazine>(*contained)
                .expect("entity in firearm should be a magazine");
            let bullets = world
                .get::<Container>(*contained)
                .expect("magazine should be a container")
                .get_entities_including_invisible();
            let magazine_name = Description::get_article_reference_name(*contained, world);

            let bullet_or_bullets = if magazine.max_bullets == 1 {
                "bullet"
            } else {
                "bullets"
            };
            format!(
                "loaded with {} containing {} of {} {}",
                magazine_name,
                bullets.len(),
                magazine.max_bullets,
                bullet_or_bullets
            )
        } else {
            panic!(
                "{} entities found in firearm {} (expected 1)",
                contents.len(),
                entity
            );
        };

        vec![
            AttributeDescription::NonSection(NonSectionAttributeDescription {
                attribute_type: NonSectionAttributeType::Is,
                description: loaded_desc,
            }),
            AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Firearm,
                attributes: vec![SectionAttributeDescription {
                    name: "Caliber".to_string(),
                    description: AmmoCaliberNameCatalog::get_value(&firearm.caliber, world),
                }],
            }),
        ]
    }
}

/// Prevents putting entities into firearms if they're not magazines of the correct caliber or if the firearm already has a  magazine in it.
fn verify_item_to_put_in_firearm(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let destination = notification.contents.destination;

    let Some(firearm) = world.get::<Firearm>(destination) else {
        return VerifyResult::valid();
    };

    let Some(magazine) = world.get::<FirearmMagazine>(notification.contents.item) else {
        return VerifyResult::invalid(
            performing_entity,
            build_incorrect_item_error(destination, firearm, performing_entity, world),
        );
    };

    let mut errors = Vec::new();

    if magazine.caliber != firearm.caliber {
        errors.push(build_incorrect_item_error(
            destination,
            firearm,
            performing_entity,
            world,
        ));
    }

    let firearm_container = world
        .get::<Container>(destination)
        .expect("destination firearm should be a container");

    if !firearm_container
        .get_entities_including_invisible()
        .is_empty()
    {
        let firearm_name =
            Description::get_reference_name(destination, Some(performing_entity), world);
        errors.push(GameMessage::Error(format!(
            "{firearm_name} is already loaded."
        )))
    }

    if !errors.is_empty() {
        return VerifyResult::invalid_with_messages([(performing_entity, errors)].into());
    }

    VerifyResult::valid()
}

/// Creates a `GameMessage` containing the error message for attempting to put the wrong thing in a firearm.
fn build_incorrect_item_error(
    firearm_entity: Entity,
    firearm: &Firearm,
    performing_entity: Entity,
    world: &World,
) -> GameMessage {
    let firearm_name =
        Description::get_reference_name(firearm_entity, Some(performing_entity), world);
    let caliber_name = AmmoCaliberNameCatalog::get_value(&firearm.caliber, world);

    GameMessage::Error(format!(
        "{firearm_name} can only be loaded with {caliber_name} magazines."
    ))
}
