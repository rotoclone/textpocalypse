use std::any::Any;

use bevy_ecs::prelude::*;

use crate::action::{Action, ActionResult};

type ActionInteractionHandler<T> =
    fn(ActionInteractionContext<T>, &mut World) -> ActionInteractionResult;

/// Info passed to an action interaction handler.
pub struct ActionInteractionContext<'a, 'b, T: Action> {
    /// The entity performing `action_1`
    pub performing_entity_1: Entity,
    /// The first action involved in the potential interaction
    pub action_1: &'a T,
    /// The entity performing `action_2`
    pub performing_entity_2: Entity,
    /// The second action involved in the potential interaction
    pub action_2: &'b dyn Action,
}

impl<'a, 'b, T: Action> ActionInteractionContext<'a, 'b, T> {
    /// Casts `action_2` to `O`, if possible.
    pub fn get_other_action_as<O: 'static>(&self) -> Option<&O> {
        let action_any = &self.action_2 as &dyn Any;
        action_any.downcast_ref()
    }
}

/// The result of attempting to have two actions interact.
pub enum ActionInteractionResult {
    /// The actions interacted and they can both be considered to have been performed
    Interacted(ActionResult, ActionResult),
    /// The actions did not interact and therefore have not been performed
    DidNotInteract,
}

impl ActionInteractionResult {
    /// Returns true if this represents a result where the actions interacted, false otherwise.
    pub fn interacted(&self) -> bool {
        match self {
            Self::Interacted(_, _) => true,
            Self::DidNotInteract => false,
        }
    }
}

//TODO doc
#[derive(Resource)]
pub struct ActionInteractionHandlers<T: Action> {
    pub handlers: Vec<ActionInteractionHandler<T>>,
}

// manually implementing `Clone` so it's implemented even if `T` isn't `Clone`
impl<T: Action> Clone for ActionInteractionHandlers<T> {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
        }
    }
}
