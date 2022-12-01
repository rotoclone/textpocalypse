use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::{
    can_receive_messages,
    component::{Description, Location, OpenState, Room},
    move_entity, Direction, GameMessage, RoomDescription,
};

use super::{Action, ActionResult};

#[derive(Debug)]
pub struct Move {
    pub direction: Direction,
}

impl Action for Move {
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let current_room_id = world
            .get::<Location>(performing_entity)
            .expect("Moving entity should have a location")
            .id;

        let current_room = world
            .get::<Room>(current_room_id)
            .expect("Moving entity's location should be a room");
        let mut messages = HashMap::new();
        let mut should_tick = false;
        let can_receive_messages = can_receive_messages(world, performing_entity);

        if let Some((connecting_entity, connection)) =
            current_room.get_connection_in_direction(&self.direction, &world)
        {
            if let Some(message) = invalid_move_message(connecting_entity, &world) {
                messages.insert(performing_entity, vec![message]);
            } else {
                let new_room_id = connection.destination;
                move_entity(performing_entity, new_room_id, world);
                should_tick = true;

                if can_receive_messages {
                    let new_room = world
                        .get::<Room>(new_room_id)
                        .expect("Destination entity should be a room");
                    let room_desc = RoomDescription::from_room(new_room, performing_entity, world);
                    messages.insert(performing_entity, vec![GameMessage::Room(room_desc)]);
                }
            }
        } else if can_receive_messages {
            messages.insert(
                performing_entity,
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

/// Determines if the provided entity can be moved through
fn invalid_move_message(entity: Entity, world: &World) -> Option<GameMessage> {
    world
        .get::<OpenState>(entity)
        .map(|state| {
            if !state.open {
                let message = world
                    .get::<Description>(entity)
                    .map_or("It's closed.".to_string(), |desc| {
                        format!("The {} is closed.", desc.name)
                    });
                Some(GameMessage::Message(message))
            } else {
                None
            }
        })
        .unwrap_or(None)
}
