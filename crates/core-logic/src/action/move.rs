use std::collections::HashMap;

use hecs::Entity;

use crate::{
    can_receive_messages, move_entity, room::Room, Direction, GameMessage, Location,
    RoomDescription, World,
};

use super::{Action, ActionResult};

#[derive(Debug)]
pub struct Move {
    pub direction: Direction,
}

impl Action for Move {
    fn perform(&self, entity_id: Entity, world: &mut World) -> ActionResult {
        let current_room_id = world.get::<&Location>(entity_id).unwrap().id;

        let current_room = world.get::<&Room>(current_room_id).unwrap();
        let mut messages = HashMap::new();
        let mut should_tick = false;
        let can_receive_messages = can_receive_messages(world, entity_id);

        if let Some(connection) = current_room.connection_in_direction(&self.direction) {
            let new_room_id = connection.destination_entity_id;
            drop(current_room);
            move_entity(world, entity_id, new_room_id);
            should_tick = true;

            if can_receive_messages {
                let new_room = world.get::<&Room>(new_room_id).unwrap();
                let room_desc = RoomDescription::from_room(&new_room, world);
                messages.insert(entity_id, vec![GameMessage::Room(room_desc)]);
            }
        } else if can_receive_messages {
            messages.insert(
                entity_id,
                vec![GameMessage::Error(
                    "You can't move in that direction.".to_string(),
                )],
            );
        }

        ActionResult {
            messages,
            should_tick,
        }
    }
}
