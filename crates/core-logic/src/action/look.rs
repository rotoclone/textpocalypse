use crate::world::LocationId;
use crate::{EntityDescription, EntityId, World};
use crate::{GameMessage, LocationDescription};

use super::{Action, ActionResult};

#[derive(Debug)]
pub struct Look {
    pub target: LookTarget,
}

#[derive(Debug)]
pub enum LookTarget {
    Entity(EntityId),
    Location(LocationId),
}

impl Action for Look {
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult {
        if !world.can_receive_messages(entity_id) {
            return ActionResult::none();
        }

        match self.target {
            LookTarget::Entity(target_id) => {
                let target = world.get_entity(target_id);
                ActionResult {
                    messages: [(
                        entity_id,
                        vec![GameMessage::Entity(EntityDescription::from_entity(target))],
                    )]
                    .into(),
                    should_tick: false,
                }
            }
            LookTarget::Location(target_id) => {
                let target = world.get_location(target_id);

                ActionResult {
                    messages: [(
                        entity_id,
                        vec![GameMessage::Location(LocationDescription::from_location(
                            target, world,
                        ))],
                    )]
                    .into(),
                    should_tick: false,
                }
            }
        }
    }
}
