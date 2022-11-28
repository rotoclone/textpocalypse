use std::collections::HashMap;

use crate::{GameMessage, World};

mod look;
use hecs::Entity;
pub use look::Look;

mod r#move;
pub use r#move::Move;

pub struct ActionResult {
    pub messages: HashMap<Entity, Vec<GameMessage>>,
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

    /// Creates an action result with a single message for an entity.
    pub fn message(entity_id: Entity, message: String) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Message(message)])].into(),
            should_tick: false,
        }
    }

    /// Creates an action result with a single error message for an entity.
    pub fn error(entity_id: Entity, message: String) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Error(message)])].into(),
            should_tick: false,
        }
    }
}

pub trait Action: std::fmt::Debug {
    /// Called when the provided entity should perform the action.
    fn perform(&self, entity_id: Entity, world: &mut World) -> ActionResult;
}
