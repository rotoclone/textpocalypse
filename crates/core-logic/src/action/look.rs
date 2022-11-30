use bevy_ecs::prelude::*;

use crate::room::Room;
use crate::{can_receive_messages, Description, EntityDescription, Name, World};
use crate::{GameMessage, RoomDescription};

use super::{Action, ActionResult};

#[derive(Debug)]
pub struct Look {
    pub target: Entity,
}

impl Action for Look {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, performing_entity) {
            return ActionResult::none();
        }

        let target = world.entity(self.target);

        if let Some(room) = target.get::<Room>() {
            return ActionResult {
                messages: [(
                    performing_entity,
                    vec![GameMessage::Room(RoomDescription::from_room(
                        room,
                        performing_entity,
                        world,
                    ))],
                )]
                .into(),
                should_tick: false,
            };
        }

        if let Some(name) = target.get::<Name>() {
            let description = target
                .get::<Description>()
                .map(|d| d.long.clone())
                .unwrap_or_default();
            return ActionResult {
                messages: [(
                    performing_entity,
                    vec![GameMessage::Entity(EntityDescription {
                        name: name.primary.clone(),
                        description,
                    })],
                )]
                .into(),
                should_tick: false,
            };
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }
}
