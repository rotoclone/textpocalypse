use bevy_ecs::prelude::*;
use std::collections::HashMap;

use crate::notification::BeforeActionNotification;
use crate::{GameMessage, World};

mod look;
pub use look::LookParser;

mod r#move;
pub use r#move::MoveAction;
pub use r#move::MoveParser;

mod open;
pub use open::OpenAction;
pub use open::OpenParser;

mod help;
pub use help::HelpParser;

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
    pub fn message(entity_id: Entity, message: String, should_tick: bool) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Message(message)])].into(),
            should_tick,
        }
    }

    /// Creates an action result with a single error message for an entity.
    pub fn error(entity_id: Entity, message: String) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Error(message)])].into(),
            should_tick: false,
        }
    }

    /// Creates an `ActionResultBuilder` with `should_tick` set to false.
    pub fn builder_no_tick() -> ActionResultBuilder {
        ActionResultBuilder {
            result: ActionResult {
                messages: HashMap::new(),
                should_tick: false,
            },
        }
    }

    /// Creates an `ActionResultBuilder` with `should_tick` set to true.
    pub fn builder_should_tick() -> ActionResultBuilder {
        ActionResultBuilder {
            result: ActionResult {
                messages: HashMap::new(),
                should_tick: false,
            },
        }
    }
}

pub struct ActionResultBuilder {
    result: ActionResult,
}

impl ActionResultBuilder {
    /// Builds the `ActionResult`.
    pub fn build(self) -> ActionResult {
        self.result
    }

    /// Adds a message to be sent to an entity.
    pub fn with_message(self, entity_id: Entity, message: String) -> ActionResultBuilder {
        self.with_game_message(entity_id, GameMessage::Message(message))
    }

    /// Adds an error message to be sent to an entity.
    pub fn with_error(self, entity_id: Entity, message: String) -> ActionResultBuilder {
        self.with_game_message(entity_id, GameMessage::Error(message))
    }

    /// Adds a `GameMessage` to be sent to an entity.
    fn with_game_message(mut self, entity_id: Entity, message: GameMessage) -> ActionResultBuilder {
        self.result
            .messages
            .entry(entity_id)
            .or_insert_with(Vec::new)
            .push(message);

        self
    }
}

pub trait Action: std::fmt::Debug + Send + Sync {
    /// Called when the provided entity should perform the action.
    fn perform(&self, performing_entity: Entity, world: &mut World) -> ActionResult;

    /// Sends a notification that this action is about to be performed.
    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    );
}
