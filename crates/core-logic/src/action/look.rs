use bevy_ecs::prelude::*;

use crate::{
    can_receive_messages,
    component::{Description, Room},
    EntityDescription, GameMessage, RoomDescription, World,
};

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

        if let Some(desc) = target.get::<Description>() {
            return ActionResult {
                messages: [(
                    performing_entity,
                    vec![GameMessage::Entity(EntityDescription::from_description(
                        desc,
                    ))],
                )]
                .into(),
                should_tick: false,
            };
        }

        ActionResult::error(performing_entity, "You can't see that.".to_string())
    }
}
