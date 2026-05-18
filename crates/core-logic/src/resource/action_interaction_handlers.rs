use std::marker::PhantomData;
use std::{collections::HashMap, hash::Hash};

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

/// An identifier for a registered action interaction handler.
///
/// This is only unique to the action type of the interaction handler.
/// For example, the first handler registered for `MoveAction` and the first one registered for `AttackAction` will both have the same internal
/// value, just different associated types.
pub struct ActionInteractionHandlerId<T> {
    value: u64,
    _t: PhantomData<fn(T)>,
}

// need to manually implement traits due to https://github.com/rust-lang/rust/issues/26925
impl<T> Clone for ActionInteractionHandlerId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ActionInteractionHandlerId<T> {}

impl<T> PartialEq for ActionInteractionHandlerId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for ActionInteractionHandlerId<T> {}

impl<T> Hash for ActionInteractionHandlerId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> ActionInteractionHandlerId<T> {
    /// Creates a new action interaction handler ID with the minimum starting value.
    fn new() -> ActionInteractionHandlerId<T> {
        ActionInteractionHandlerId {
            value: 0,
            _t: PhantomData,
        }
    }

    /// Increments this action interaction handler ID's value.
    fn next(mut self) -> ActionInteractionHandlerId<T> {
        self.value += 1;
        self
    }
}

/// The set of interaction handlers for a specific action type.
#[derive(Resource)]
pub struct ActionInteractionHandlers<T: Action> {
    /// The ID to be assigned to the next registered handler.
    next_id: ActionInteractionHandlerId<T>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<ActionInteractionHandlerId<T>, ActionInteractionHandler<T>>,
}

impl<T: Action> ActionInteractionHandlers<T> {
    /// Returns true if there are no handlers registered, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Gets all the registered handlers.
    pub fn get_handlers(&self) -> Vec<ActionInteractionHandler<T>> {
        self.handlers.values().copied().collect()
    }
}
