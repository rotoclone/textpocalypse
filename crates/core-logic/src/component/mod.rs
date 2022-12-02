use bevy_ecs::prelude::*;

use crate::action::Action;
use crate::command::CommandParser;

mod door;
pub use door::Door;
pub use door::DoorBundle;

mod description;
pub use description::Description;
pub use description::EntityDescription;
pub use description::RoomConnectionEntityDescription;
pub use description::RoomEntityDescription;
pub use description::RoomLivingEntityDescription;
pub use description::RoomObjectDescription;

mod location;
pub use location::Location;

mod message_channel;
pub use message_channel::MessageChannel;

mod room;
pub use room::ExitDescription;
pub use room::Room;
pub use room::RoomDescription;

mod connection;
pub use connection::Connection;
pub use connection::Direction;

mod open_state;
pub use open_state::OpenState;

/// Trait for components that parse commands.
pub trait ParseCommand {
    /// Registers the command parser for this component on the provided entity.
    fn register_command_parser(entity: Entity, world: &mut World) {
        if let Some(mut command_parser) = world.get_mut::<CommandParser>(entity) {
            command_parser.parse_fns.push(Self::parse_command);
        } else {
            world.entity_mut(entity).insert(Self::new_command_parser());
        }
    }

    /// Creates a `CommandParser` with the parser for this component.
    fn new_command_parser() -> CommandParser {
        CommandParser {
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
