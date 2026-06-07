use std::sync::LazyLock;

use bevy_ecs::prelude::*;
use strum::EnumIter;

use crate::{
    action::Action,
    command_format::{
        entity_part_builder, literal_part, validate_parsed_value_has_component, CommandFormat,
        CommandPartId,
    },
    component::{
        description::NonSectionAttributeDescription, AttributeDescriber, AttributeDetailLevel,
        Container, DescribeAttributes, FirearmMagazine, ParseCustomInput,
        SectionAttributeDescription,
    },
    input_parser::{InputParseError, InputParser},
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    AttributeDescription, AttributeSection, AttributeSectionName, NonSectionAttributeType,
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
    /// The magazine currently loaded into this firearm, if any
    pub magazine: Option<Entity>,
}

impl ParseCustomInput for Firearm {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(UnloadParser)]
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
        //TODO
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

        let caliber_name = AmmoCaliberNameCatalog::get_value(&firearm.caliber, world);

        let loaded_desc = if let Some(magazine_entity) = firearm.magazine {
            let bullets = world
                .get::<Container>(magazine_entity)
                .expect("magazine should be a container")
                .get_entities_including_invisible();

            let magazine_desc = if bullets.is_empty() {
                "an empty magazine".to_string()
            } else {
                let bullet_or_bullets = if bullets.len() == 1 {
                    "bullet"
                } else {
                    "bullets"
                };
                format!("{} {} {}", bullets.len(), caliber_name, bullet_or_bullets)
            };

            format!("loaded with {magazine_desc}")
        } else {
            "unloaded".to_string()
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
                    description: caliber_name,
                }],
            }),
        ]
    }
}

static UNLOAD_TARGET_PART_ID: CommandPartId<Entity> = CommandPartId::new("target");

static UNLOAD_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("unload"))
        .then(literal_part(" "))
        .then(
            entity_part_builder(UNLOAD_TARGET_PART_ID)
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<Firearm>(context, "unload", world)
                })
                .build()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("gun"),
        )
});

struct UnloadParser;

impl InputParser for UnloadParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        todo!() //TODO
    }

    fn get_input_formats(&self) -> Vec<String> {
        todo!() //TODO
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
