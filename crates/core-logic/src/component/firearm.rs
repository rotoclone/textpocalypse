use std::sync::LazyLock;

use bevy_ecs::prelude::*;

use crate::{
    command_format::{
        entity_part_builder, literal_part, validate_parsed_value_has_component, CommandFormat,
        CommandPartId,
    },
    component::{AttributeDescriber, DescribeAttributes, ParseCustomInput},
    input_parser::InputParser,
};

/// Component for entities that are firearms.
#[derive(Component)]
pub struct Firearm;

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
        pov_entity: Entity,
        entity: Entity,
        detail_level: super::AttributeDetailLevel,
        world: &World,
    ) -> Vec<super::AttributeDescription> {
        todo!() //TODO
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
    ) -> Result<Box<dyn crate::action::Action>, crate::input_parser::InputParseError> {
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
