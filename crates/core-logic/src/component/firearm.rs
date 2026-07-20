use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use core_logic_derive::ActionBoilerplate;
use strum::EnumIter;

use crate::{
    action::{
        Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ActionTag,
        AttackAction, PutAction,
    },
    command_format::{
        entity_part_builder, literal_part, validate_parsed_value_has_component, CommandFormat,
        CommandPartId,
    },
    component::{
        description::NonSectionAttributeDescription, ActionQueue, AfterActionPerformNotification,
        AttributeDescriber, AttributeDetailLevel, Bullet, Container, DescribeAttributes,
        Description, FirearmMagazine, Location, ParseCustomInput, SectionAttributeDescription,
        VerifyActionNotification, VerifyResult,
    },
    despawn_entity,
    input_parser::{InputParseError, InputParser},
    notification::{Notification, NotificationHandlers, ReturningNotificationHandlers},
    resource::{
        catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
        spawner::{BulletCasingSpawner, Spawner},
    },
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
        vec![Box::new(ReloadFirearmParser)]
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
        ReturningNotificationHandlers::add_handler(verify_firearm_loaded, world);
        NotificationHandlers::add_handler(use_bullet_on_shoot, world);
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

        let num_bullets;
        let max_bullets;

        let loaded_desc = if contents.is_empty() {
            num_bullets = 0;
            max_bullets = None;
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

            num_bullets = bullets.len();
            max_bullets = Some(magazine.max_bullets);

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

        let ammo_desc = if let Some(max_bullets) = max_bullets {
            format!("{num_bullets}/{max_bullets}")
        } else {
            num_bullets.to_string()
        };

        vec![
            AttributeDescription::NonSection(NonSectionAttributeDescription {
                attribute_type: NonSectionAttributeType::Is,
                description: loaded_desc,
            }),
            AttributeDescription::Section(AttributeSection {
                name: AttributeSectionName::Firearm,
                attributes: vec![
                    SectionAttributeDescription {
                        name: "Caliber".to_string(),
                        description: AmmoCaliberNameCatalog::get_value(&firearm.caliber, world),
                    },
                    SectionAttributeDescription {
                        name: "Ammo".to_string(),
                        description: ammo_desc,
                    },
                ],
            }),
        ]
    }
}

static FIREARM_PART_ID: CommandPartId<Entity> = CommandPartId::new("firearm");
static MAG_PART_ID: CommandPartId<Entity> = CommandPartId::new("magazine");

static RELOAD_FIREARM_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("reload"))
        .then(literal_part(" "))
        .then(
            entity_part_builder(FIREARM_PART_ID)
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<Firearm>(context, "reload", world)
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("firearm"),
        )
        .then(literal_part(" "))
        .then(literal_part("with"))
        .then(literal_part(" "))
        .then(
            entity_part_builder(MAG_PART_ID)
                .with_validator(|context, world| {
                    //TODO verify the magazine is the same caliber as the firearm
                    validate_parsed_value_has_component::<FirearmMagazine>(
                        context,
                        "reload with",
                        world,
                    )
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("magazine"),
        )
});

struct ReloadFirearmParser;

impl InputParser for ReloadFirearmParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = RELOAD_FIREARM_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(ReloadFirearmAction {
            firearm: parsed.get(FIREARM_PART_ID),
            magazine: parsed.get(MAG_PART_ID),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![RELOAD_FIREARM_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        if world.get::<Firearm>(entity).is_some() {
            vec![RELOAD_FIREARM_FORMAT
                .get_format_description()
                .with_targeted_entity(FIREARM_PART_ID, entity, world)
                .to_string()]
        } else if world.get::<FirearmMagazine>(entity).is_some() {
            vec![RELOAD_FIREARM_FORMAT
                .get_format_description()
                .with_targeted_entity(MAG_PART_ID, entity, world)
                .to_string()]
        } else {
            Vec::new()
        }
    }
}

/// Makes an entity reload a firearm.
#[derive(ActionBoilerplate, Debug)]
pub struct ReloadFirearmAction {
    /// The firearm to reload
    pub firearm: Entity,
    /// The new magazine to load into the firearm.
    pub magazine: Entity,
    /// The notification sender
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for ReloadFirearmAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let magazine_location = world
            .get::<Location>(self.magazine)
            .expect("magazine should have a location");

        // queue actions backwards so they end up being performed in the correct order

        // put the new mag in the firearm
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(PutAction {
                item: self.magazine,
                source: magazine_location.id,
                destination: self.firearm,
                notification_sender: ActionNotificationSender::new(),
            }),
        );

        // take the old mag out of the firearm
        let firearm_container = world
            .get::<Container>(self.firearm)
            .expect("firearm should be a container");
        let firearm_contents = firearm_container.get_entities_including_invisible();
        if firearm_contents.is_empty() {
            return ActionResult::none();
        }

        if firearm_contents.len() > 1 {
            panic!(
                "Expected 1 item in firearm, but found {}",
                firearm_contents.len()
            );
        }

        // unwrap is safe due to length check above
        let old_mag = firearm_contents.iter().next().unwrap();
        ActionQueue::queue_first(
            world,
            performing_entity,
            Box::new(PutAction {
                item: *old_mag,
                source: self.firearm,
                destination: performing_entity,
                notification_sender: ActionNotificationSender::new(),
            }),
        );

        ActionResult::none()
    }

    fn interrupt(&self, _: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::none()
    }

    fn may_require_tick(&self) -> bool {
        false
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        HashSet::new()
    }

    fn get_interaction_target(&self, _: &World) -> Option<Entity> {
        None
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

/// Prevents firing a gun if it doesn't have at least one bullet in it.
fn verify_firearm_loaded(
    notification: &Notification<VerifyActionNotification, AttackAction>,
    world: &World,
) -> VerifyResult {
    let chosen_weapon = notification.contents.weapon;
    let attacker = notification.notification_type.performing_entity;

    let Some(weapon_entity) = chosen_weapon.get_entity::<AttackAction>(attacker, world) else {
        return VerifyResult::valid();
    };

    if world.get::<Firearm>(weapon_entity).is_none() {
        return VerifyResult::valid();
    };

    if let Some(magazine_entity) = get_magazine(weapon_entity, world) {
        let magazine_container = world
            .get::<Container>(magazine_entity)
            .expect("magazine should be a container");
        if magazine_container
            .get_entities_including_invisible()
            .is_empty()
        {
            // has an empty magazine
            return VerifyResult::invalid(
                attacker,
                build_unloaded_error(weapon_entity, attacker, world),
            );
        }
    } else {
        // doesn't have a magazine
        return VerifyResult::invalid(
            attacker,
            build_unloaded_error(weapon_entity, attacker, world),
        );
    }

    VerifyResult::valid()
}

fn build_unloaded_error(firearm: Entity, performing_entity: Entity, world: &World) -> GameMessage {
    let firearm_name = Description::get_reference_name(firearm, Some(performing_entity), world);
    GameMessage::Error(format!("{firearm_name} isn't loaded."))
}

/// Removes a bullet from the magazine when a gun is fired, turns it into an empty casing, and drops it on the ground.
fn use_bullet_on_shoot(
    notification: &Notification<AfterActionPerformNotification, AttackAction>,
    world: &mut World,
) {
    let chosen_weapon = notification.contents.weapon;
    let attacker = notification.notification_type.performing_entity;

    let Some(weapon_entity) = chosen_weapon.get_entity::<AttackAction>(attacker, world) else {
        return;
    };

    if world.get::<Firearm>(weapon_entity).is_none() {
        return;
    };

    let Some(magazine_entity) = get_magazine(weapon_entity, world) else {
        return;
    };

    let mut magazine_container = world
        .get_mut::<Container>(magazine_entity)
        .expect("magazine should be a container");

    let Some(fired_entity) = magazine_container
        .get_entities_including_invisible_mut()
        .back()
        .copied()
    else {
        return;
    };

    let bullet_caliber = world
        .get::<Bullet>(fired_entity)
        .expect("fired entity should be a bullet")
        .caliber
        .clone();

    let attacker_location = *world
        .get::<Location>(attacker)
        .expect("attacker should have a location");

    BulletCasingSpawner::spawn(bullet_caliber, attacker_location.id, world);
    despawn_entity(fired_entity, world);
}

/// Gets the magazine loaded in a firearm, if there is one.
fn get_magazine(firearm: Entity, world: &World) -> Option<Entity> {
    let firearm_container = world.get::<Container>(firearm)?;

    let contents = firearm_container.get_entities_including_invisible();
    if contents.len() > 1 {
        panic!(
            "Firearm contains {} entities (expected 0 or 1)",
            contents.len()
        );
    }

    contents.iter().next().copied()
}
