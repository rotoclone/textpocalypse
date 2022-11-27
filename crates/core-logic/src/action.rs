use std::collections::HashMap;

use crate::{Direction, EntityId, GameMessage, LocationDescription, World};

pub struct ActionResult {
    pub messages: HashMap<EntityId, Vec<GameMessage>>,
    pub should_tick: bool,
}

pub trait Action {
    /// Called when the provided entity performs the action.
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult;
}

#[derive(Debug)]
pub struct Move {
    pub direction: Direction,
}

impl Action for Move {
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult {
        let entity = world.get_entity(entity_id);

        let current_location_id = entity.get_location_id();
        let current_location = world.get_location(current_location_id);
        let mut messages = HashMap::new();
        let mut should_tick = false;
        let can_receive_messages = world.can_receive_messages(entity_id);

        if let Some(connection) = current_location.connections.get(&self.direction) {
            let new_location_id = connection.location_id;
            world.move_entity(entity_id, current_location_id, new_location_id);
            should_tick = true;

            if can_receive_messages {
                let new_location = world.get_location(new_location_id);
                let location_desc = LocationDescription::from_location(new_location, world);
                messages.insert(entity_id, vec![GameMessage::Location(location_desc)]);
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

#[derive(Debug)]
pub struct Look;

impl Action for Look {
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult {
        if !world.can_receive_messages(entity_id) {
            return ActionResult {
                messages: HashMap::new(),
                should_tick: false,
            };
        }

        let entity = world.get_entity(entity_id);
        let current_location = world.get_location(entity.get_location_id());

        ActionResult {
            messages: [(
                entity_id,
                vec![GameMessage::Location(LocationDescription::from_location(
                    current_location,
                    world,
                ))],
            )]
            .into(),
            should_tick: false,
        }
    }
}
