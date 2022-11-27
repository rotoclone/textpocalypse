use std::collections::HashMap;

use crate::{EntityId, GameMessage, World};

mod look;
pub use look::Look;

mod r#move;
pub use r#move::Move;

pub struct ActionResult {
    pub messages: HashMap<EntityId, Vec<GameMessage>>,
    pub should_tick: bool,
}

pub trait Action: std::fmt::Debug {
    /// Called when the provided entity should perform the action.
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult;
}
