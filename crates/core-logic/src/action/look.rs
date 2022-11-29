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
    fn perform(&self, entity_id: Entity, world: &mut World) -> ActionResult {
        if !can_receive_messages(world, entity_id) {
            return ActionResult::none();
        }

        if let Some(room) = world.get::<Room>(self.target) {
            return ActionResult {
                messages: [(
                    entity_id,
                    vec![GameMessage::Room(RoomDescription::from_room(room, world))],
                )]
                .into(),
                should_tick: false,
            };
        }

        let entity = world.entity(entity_id);
        if let Some(name) = entity.get::<Name>() {
            let description = entity
                .get::<Description>()
                .map(|d| d.long.clone())
                .unwrap_or_default();
            return ActionResult {
                messages: [(
                    entity_id,
                    vec![GameMessage::Entity(EntityDescription {
                        name: name.0.clone(),
                        description,
                    })],
                )]
                .into(),
                should_tick: false,
            };
        }

        ActionResult::error(entity_id, "You can't see that.".to_string())
    }
}
