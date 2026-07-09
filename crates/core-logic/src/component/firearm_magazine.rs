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
    dynamic_message::{DynamicMessage, DynamicMessageLocation},
    input_parser::{find_entities_in_presence_of, InputParser},
    message_format::{BasicTokens, MessageFormat},
    move_entity,
    notification::{Notification, ReturningNotificationHandlers},
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    send_message, AttributeDescription, AttributeSection, AttributeSectionName, GameMessage,
    InternalMessageCategory, MessageCategory, MessageDelay, SurroundingsMessageCategory,
};

/// Component for entities that can be loaded into firearms.
#[derive(Component)]
pub struct FirearmMagazine {
    /// The caliber of bullet that fits in this magazine
    pub caliber: AmmoCaliber,
    /// The maximum number of bullets this magazine can hold at once
    pub max_bullets: usize,
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
        let parsed = FILL_MAG_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(FillMagazineAction {
            magazine: parsed.get(MAG_PART_ID),
            bullet: parsed.get(BULLET_PART_ID),
            loaded_any: false,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![FILL_MAG_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        if world.get::<FirearmMagazine>(entity).is_some() {
            vec![FILL_MAG_FORMAT
                .get_format_description()
                .with_targeted_entity(MAG_PART_ID, entity, world)
                .to_string()]
        } else if world.get::<Bullet>(entity).is_some() {
            vec![FILL_MAG_FORMAT
                .get_format_description()
                .with_targeted_entity(BULLET_PART_ID, entity, world)
                .to_string()]
        } else {
            Vec::new()
        }
    }
}

/// Makes an entity fill a magazine with bullets.
#[derive(ActionBoilerplate, Debug)]
pub struct FillMagazineAction {
    /// The magazine to fill
    pub magazine: Entity,
    /// The first bullet to put in the magazine
    pub bullet: Entity,
    /// Whether any bullets have been loaded yet. Should start false.
    pub loaded_any: bool,
    /// The notification sender
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for FillMagazineAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let source_bullet_name = Description::get_name(self.bullet, world);
        let source_bullet_caliber = &world
            .get::<Bullet>(self.bullet)
            .expect("bullet should be a bullet")
            .caliber;

        let magazine = world
            .get::<FirearmMagazine>(self.magazine)
            .expect("magazine should be a magazine");

        let magazine_container = world
            .get::<Container>(self.magazine)
            .expect("magazine should be a container");

        let starting_num_bullets_loaded =
            magazine_container.get_entities_including_invisible().len();
        let max_bullets = magazine.max_bullets;

        if starting_num_bullets_loaded >= max_bullets {
            let magazine_name =
                Description::get_reference_name(self.magazine, Some(performing_entity), world);
            return ActionResult::error(performing_entity, format!("{magazine_name} is full."));
        }

        let candidate_entities = find_entities_in_presence_of(performing_entity, world);
        let mut candidate_bullets = candidate_entities
            .iter()
            .copied()
            .filter(|e| {
                *e == self.bullet
                    || (Description::get_name(*e, world) == source_bullet_name
                        && world
                            .get::<Bullet>(*e)
                            .is_some_and(|b| b.caliber == *source_bullet_caliber))
            })
            .collect::<Vec<Entity>>();
        let Some(bullet) = candidate_bullets.pop() else {
            let message = if self.loaded_any {
                "No more matching bullets found.".to_string()
            } else {
                "No matching bullets found.".to_string()
            };

            return ActionResult::error(performing_entity, message);
        };

        let mut result_builder = ActionResult::builder();

        move_entity(bullet, self.magazine, world);
        self.loaded_any = true;
        //TODO this message only gets sent once for some reason, even if multiple bullets are loaded
        result_builder = result_builder.with_dynamic_message(
            Some(performing_entity),
            DynamicMessageLocation::SourceEntity,
            DynamicMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
                MessageFormat::new(
                    "${entity.Name} ${entity.you:put/puts} ${bullet.a} ${bullet.plain_name} into ${magazine.name}.",
                )
                .expect("message format should be valid"),
                BasicTokens::new()
                    .with_entity("entity".into(), performing_entity)
                    .with_entity("magazine".into(), self.magazine)
                    .with_entity("bullet".into(), bullet),
            ),
            world,
        );

        if starting_num_bullets_loaded + 1 == max_bullets {
            // this was the last bullet the magazine can hold
            let magazine_name =
                Description::get_reference_name(self.magazine, Some(performing_entity), world);
            return result_builder
                .with_message(
                    performing_entity,
                    format!("{magazine_name} is now full."),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::None,
                )
                .build_complete_should_tick(true);
        } else if candidate_bullets.is_empty() {
            // this was the last bullet available to be loaded
            return result_builder
                .with_message(
                    performing_entity,
                    "That was the last bullet you could find.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::None,
                )
                .build_complete_should_tick(true);
        }

        ActionResult::builder().build_incomplete(true)
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

//TODO verify performing entity has access to the mag to fill and the bullet

//TODO verify fill action has bullet and mag of matching caliber

//TODO verify fill action has non-full mag

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

    if magazine_container.get_entities_including_invisible().len() >= magazine.max_bullets {
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
