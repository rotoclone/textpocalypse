use std::any::Any;

use bevy_ecs::prelude::*;

use crate::action::{Action, ActionResult};

type ActionInteractionHandler<T> =
    fn(ActionInteractionContext<T>, &mut World) -> ActionInteractionResult;

/// Info passed to an action interaction handler.
pub struct ActionInteractionContext<T: Action> {
    /// The entity performing `action_1`
    performing_entity_1: Entity,
    /// The first action involved in the potential interaction
    action_1: T,
    /// The entity performing `action_2`
    performing_entity_2: Entity,
    /// The second action involved in the potential interaction
    action_2: Box<dyn Action>,
}

impl<T: Action> ActionInteractionContext<T> {
    /// Casts `action_2` to `O`, if possible.
    pub fn get_other_action_as<O: 'static>(&self) -> Option<&O> {
        let action_any = &self.action_2 as &dyn Any;
        action_any.downcast_ref()
    }
}

/// The result of attempting to have two actions interact.
pub enum ActionInteractionResult {
    /// The actions interacted and they can both be considered to have been performed
    Interacted((ActionResult, ActionResult)),
    /// The actions did not interact and therefore have not been performed
    DidNotInteract,
}

//TODO doc
#[derive(Resource)]
pub struct ActionInteractionHandlers<T: Action> {
    handlers: Vec<ActionInteractionHandler<T>>,
}
