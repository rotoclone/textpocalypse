use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;

use core_logic_derive::ActionBoilerplate;
use nonempty::nonempty;

use crate::{
    action::{
        Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ActionTag, PutAction,
    },
    command_format::{
        entity_part_builder, literal_part, one_of_literal_part,
        validate_parsed_value_has_component, CommandFormat, CommandPartId,
    },
    component::{
        AmmoCaliber, AttributeDescriber, AttributeDetailLevel, Bullet, Container,
        DescribeAttributes, Description, ParseCustomInput, SectionAttributeDescription,
        VerifyActionNotification, VerifyResult,
    },
    input_parser::InputParser,
    notification::{Notification, ReturningNotificationHandlers},
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    AttributeDescription, AttributeSection, AttributeSectionName, GameMessage,
    InternalMessageCategory, MessageCategory, MessageDelay,
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
        vec![Box::new(FillMagazineParser)]
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

static MAG_PART_ID: CommandPartId<Entity> = CommandPartId::new("magazine");
static BULLET_PART_ID: CommandPartId<Entity> = CommandPartId::new("bullet");

static FILL_MAG_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty!["fill", "load"]))
        .then(literal_part(" "))
        .then(
            entity_part_builder(MAG_PART_ID)
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<FirearmMagazine>(
                        context,
                        "load bullets into",
                        world,
                    )
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("magazine"),
        )
        .then(literal_part(" "))
        .then(literal_part("with"))
        .then(literal_part(" "))
        .then(
            entity_part_builder(BULLET_PART_ID)
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<Bullet>(
                        context,
                        "load a magazine with",
                        world,
                    )
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("bullet"),
        )
});

struct FillMagazineParser;

impl InputParser for FillMagazineParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn crate::action::Action>, crate::input_parser::InputParseError> {
        todo!() //TODO
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![FILL_MAG_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        pov_entity: Entity,
        world: &World,
    ) -> Vec<String> {
        todo!() //TODO
    }
}

/// Makes an entity fill a magazine with bullets.
#[derive(ActionBoilerplate, Debug)]
pub struct FillMagazineAction {
    pub magazine: Entity,
    pub bullet: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for FillMagazineAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        todo!() //TODO
    }

    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult {
        let magazine_name =
            Description::get_reference_name(self.magazine, Some(performing_entity), world);

        ActionInterruptResult::message(
            performing_entity,
            format!("You stop putting bullets in {magazine_name}."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        HashSet::new()
    }

    fn get_interaction_target(&self, _: &World) -> Option<Entity> {
        None
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
