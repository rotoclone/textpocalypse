use bevy_ecs::prelude::*;

use crate::action::Action;

type CommandParserFn = fn(Entity, &str, Entity, &World) -> Option<Box<dyn Action>>;

#[derive(Component)]
pub struct CustomCommandParser {
    pub parse_fns: Vec<CommandParserFn>,
}

/// Trait for components that parse commands.
pub trait ParseCustomCommand {
    /// Registers the custom command parser for this component on the provided entity.
    fn register_command_parser(entity: Entity, world: &mut World) {
        if let Some(mut command_parser) = world.get_mut::<CustomCommandParser>(entity) {
            command_parser.parse_fns.push(Self::parse_command);
        } else {
            world.entity_mut(entity).insert(Self::new_command_parser());
        }
    }

    /// Creates a `CustomCommandParser` with the parser for this component.
    fn new_command_parser() -> CustomCommandParser {
        CustomCommandParser {
            parse_fns: vec![Self::parse_command],
        }
    }

    /// Parses the provided input into an applicable action.
    ///
    /// It should be assumed that it has already been confirmed that the commanding entity has access to this entity in order to perform actions on it before this function is called;
    /// e.g. it is in the same room as the commanding entity, or in the commanding entity's inventory.
    fn parse_command(
        this_entity_id: Entity,
        input: &str,
        commanding_entity_id: Entity,
        world: &World,
    ) -> Option<Box<dyn Action>>;
}
