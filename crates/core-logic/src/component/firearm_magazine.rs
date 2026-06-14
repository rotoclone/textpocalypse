use bevy_ecs::prelude::*;

use crate::{
    action::PutAction,
    component::{
        AmmoCaliber, AttributeDescriber, AttributeDetailLevel, Bullet, Container,
        DescribeAttributes, Description, ParseCustomInput, SectionAttributeDescription,
        VerifyActionNotification, VerifyResult,
    },
    input_parser::InputParser,
    notification::{Notification, ReturningNotificationHandlers},
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    AttributeDescription, AttributeSection, AttributeSectionName, GameMessage,
};

/// Component for entities that can be loaded into firearms.
#[derive(Component)]
pub struct FirearmMagazine {
    /// The caliber of bullet that fits in this magazine
    pub caliber: AmmoCaliber,
    /// The maximum number of bullets this magazine can hold at once
    pub max_bullets: u16,
}

impl ParseCustomInput for FirearmMagazine {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![] //TODO add fill action
    }
}

impl DescribeAttributes for FirearmMagazine {
    fn get_attribute_describer() -> Box<dyn AttributeDescriber> {
        Box::new(FirearmMagazineAttributeDescriber)
    }
}

impl FirearmMagazine {
    /// Registers handlers for magazine actions.
    pub fn register_handlers(world: &mut World) {
        ReturningNotificationHandlers::add_handler(verify_item_to_put_in_magazine, world);
    }
}

/// Describes a magazine.
#[derive(Debug)]
struct FirearmMagazineAttributeDescriber;

impl AttributeDescriber for FirearmMagazineAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        let Some(magazine) = world.get::<FirearmMagazine>(entity) else {
            return Vec::new();
        };

        vec![AttributeDescription::Section(AttributeSection {
            name: AttributeSectionName::FirearmMagazine,
            attributes: vec![
                SectionAttributeDescription {
                    name: "Caliber".to_string(),
                    description: AmmoCaliberNameCatalog::get_value(&magazine.caliber, world),
                },
                SectionAttributeDescription {
                    name: "Capacity".to_string(),
                    description: magazine.max_bullets.to_string(),
                },
            ],
        })]
    }
}

/// Prevents putting entities into magazines if they're not bullets of the correct caliber or if the magazine is already full.
fn verify_item_to_put_in_magazine(
    notification: &Notification<VerifyActionNotification, PutAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let destination = notification.contents.destination;

    let Some(magazine) = world.get::<FirearmMagazine>(destination) else {
        return VerifyResult::valid();
    };

    let Some(bullet) = world.get::<Bullet>(notification.contents.item) else {
        return VerifyResult::invalid(
            performing_entity,
            build_incorrect_item_error(destination, magazine, performing_entity, world),
        );
    };

    let mut errors = Vec::new();

    if bullet.caliber != magazine.caliber {
        errors.push(build_incorrect_item_error(
            destination,
            magazine,
            performing_entity,
            world,
        ));
    }

    let magazine_container = world
        .get::<Container>(destination)
        .expect("destination magazine should be a container");

    if magazine_container.get_entities_including_invisible().len() >= magazine.max_bullets.into() {
        let magazine_name =
            Description::get_reference_name(destination, Some(performing_entity), world);
        errors.push(GameMessage::Error(format!("{magazine_name} is full.")))
    }

    if !errors.is_empty() {
        return VerifyResult::invalid_with_messages([(performing_entity, errors)].into());
    }

    VerifyResult::valid()
}

/// Creates a `GameMessage` containing the error message for attempting to put the wrong thing in a magazine.
fn build_incorrect_item_error(
    magazine_entity: Entity,
    magazine: &FirearmMagazine,
    performing_entity: Entity,
    world: &World,
) -> GameMessage {
    let magazine_name =
        Description::get_reference_name(magazine_entity, Some(performing_entity), world);
    let caliber_name = AmmoCaliberNameCatalog::get_value(&magazine.caliber, world);

    GameMessage::Error(format!(
        "{magazine_name} can only hold {caliber_name} bullets."
    ))
}
