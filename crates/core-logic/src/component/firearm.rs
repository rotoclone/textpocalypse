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
        Container, DescribeAttributes, Description, FirearmMagazine, ParseCustomInput,
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

        let container = world
            .get::<Container>(entity)
            .expect("firearm should be a container");

        let contents = container.get_entities_including_invisible();

        let loaded_desc = if contents.len() == 0 {
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

//TODO add verification handler to only allow putting a single magazine in a gun, similar to the one for magazines with bullets

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
