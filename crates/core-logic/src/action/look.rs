use hecs::Entity;

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

        if let Ok(room) = world.get::<&Room>(self.target) {
            return ActionResult {
                messages: [(
                    entity_id,
                    vec![GameMessage::Room(RoomDescription::from_room(&room, world))],
                )]
                .into(),
                should_tick: false,
            };
        }

        if let Ok(mut query) = world.query_one::<(&Name, &Description)>(self.target) {
            if let Some((name, description)) = query.get() {
                return ActionResult {
                    messages: [(
                        entity_id,
                        vec![GameMessage::Entity(EntityDescription {
                            name: name.0.clone(),
                            description: description.long.clone(),
                        })],
                    )]
                    .into(),
                    should_tick: false,
                };
            }
        }

        ActionResult::error(entity_id, "You can't see that.".to_string())
    }
}
