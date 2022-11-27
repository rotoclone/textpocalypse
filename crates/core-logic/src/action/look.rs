use std::collections::HashMap;

use crate::{EntityId, World};
use crate::{GameMessage, LocationDescription};

use super::{Action, ActionResult};

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
