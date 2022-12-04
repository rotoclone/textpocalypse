use bevy_ecs::prelude::*;

use crate::command_parser::CommandParser;

#[derive(Component)]
pub struct CustomCommandParser {
    pub parsers: Vec<Box<dyn CommandParser>>,
}

/// Trait for components that parse commands.
pub trait ParseCustomCommand {
    /// Registers the custom command parser for this component on the provided entity.
    fn register_command_parser(entity: Entity, world: &mut World) {
        if let Some(mut command_parser) = world.get_mut::<CustomCommandParser>(entity) {
            command_parser.parsers.push(Self::get_parser());
        } else {
            world.entity_mut(entity).insert(Self::new_command_parser());
        }
    }

    /// Creates a `CustomCommandParser` with the parser for this component.
    fn new_command_parser() -> CustomCommandParser {
        CustomCommandParser {
            parsers: vec![Self::get_parser()],
        }
    }

    /// Returns the `CommandParser` for this component.
    fn get_parser() -> Box<dyn CommandParser>;
}
