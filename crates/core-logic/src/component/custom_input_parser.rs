use bevy_ecs::prelude::*;

use crate::input_parser::InputParser;

#[derive(Component)]
pub struct CustomInputParser {
    pub parsers: Vec<Box<dyn InputParser>>,
}

/// Trait for components that parse input.
pub trait ParseCustomInput {
    /// Registers the custom input parser for this component on the provided entity.
    fn register_custom_input_parser(entity: Entity, world: &mut World) {
        if let Some(mut input_parser) = world.get_mut::<CustomInputParser>(entity) {
            input_parser.parsers.extend(Self::get_parsers());
        } else {
            world
                .entity_mut(entity)
                .insert(Self::new_custom_input_parser());
        }

        Self::register_handlers(world);
    }

    /// Creates a `CustomInputParser` with the parser for this component.
    fn new_custom_input_parser() -> CustomInputParser {
        CustomInputParser {
            parsers: Self::get_parsers(),
        }
    }

    /// Returns the `InputParser` for this component.
    fn get_parsers() -> Vec<Box<dyn InputParser>>;

    /// Registers any notification handlers needed for this component.
    fn register_handlers(_world: &mut World) {}
}
