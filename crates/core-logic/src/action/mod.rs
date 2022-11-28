use std::collections::HashMap;

use crate::{EntityId, GameMessage, World};

mod look;
pub use look::Look;
pub use look::LookTarget;

mod r#move;
pub use r#move::Move;

pub struct ActionResult {
    pub messages: HashMap<EntityId, Vec<GameMessage>>,
    pub should_tick: bool,
}

impl ActionResult {
    /// Creates an action result signifying that nothing of note occurred.
    pub fn none() -> ActionResult {
        ActionResult {
            messages: HashMap::new(),
            should_tick: false,
        }
    }
}

pub trait Action: std::fmt::Debug {
    /// Called when the provided entity should perform the action.
    fn perform(&self, entity_id: EntityId, world: &mut World) -> ActionResult;
}
