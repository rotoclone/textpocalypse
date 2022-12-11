use bevy_ecs::prelude::*;
use std::collections::HashMap;

use crate::BeforeActionNotification;
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

mod wait;
pub use wait::WaitParser;

/// The result of a single tick of an action being performed.
pub struct ActionResult {
    /// Any messages that should be sent.
    pub messages: HashMap<Entity, Vec<GameMessage>>,
    /// Whether a tick should happen due to the action being performed.
    pub should_tick: bool,
    /// Whether the action is now complete. If this is false, `perform` will be called on the action again.
    pub is_complete: bool,
}

impl ActionResult {
    /// Creates an action result signifying that nothing of note occurred.
    pub fn none() -> ActionResult {
        ActionResult {
            messages: HashMap::new(),
            should_tick: false,
            is_complete: true,
        }
    }

    /// Creates an action result with a single message for an entity, denoting that the action is complete.
    pub fn message(entity_id: Entity, message: String, should_tick: bool) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Message(message)])].into(),
            should_tick,
            is_complete: true,
        }
    }

    /// Creates an action result with a single error message for an entity, denoting that the action is complete and a tick should not happen.
    pub fn error(entity_id: Entity, message: String) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Error(message)])].into(),
            should_tick: false,
            is_complete: true,
        }
    }

    /// Creates an `ActionResultBuilder`.
    pub fn builder() -> ActionResultBuilder {
        ActionResultBuilder {
            result: ActionResult::none(),
        }
    }
}

pub struct ActionResultBuilder {
    result: ActionResult,
}

impl ActionResultBuilder {
    /// Builds the `ActionResult`, denoting that the action has been completed and a tick should happen.
    pub fn build_complete_should_tick(mut self) -> ActionResult {
        self.result.should_tick = true;
        self.result.is_complete = true;
        self.result
    }

    /// Builds the `ActionResult`, denoting that the action has been completed and a tick should not happen.
    pub fn build_complete_no_tick(mut self) -> ActionResult {
        self.result.should_tick = false;
        self.result.is_complete = true;
        self.result
    }

    /// Builds the `ActionResult`, denoting that the action has not been completed.
    pub fn build_incomplete(mut self) -> ActionResult {
        self.result.should_tick = true;
        self.result.is_complete = false;
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
    /// Called when the provided entity should perform one tick of the action.
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult;

    /// Sends a notification that this action is about to be performed.
    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    );
}
