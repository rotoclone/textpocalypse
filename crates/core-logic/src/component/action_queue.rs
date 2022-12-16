use std::collections::{HashMap, VecDeque};

use bevy_ecs::prelude::*;
use log::debug;

use crate::{
    action::Action, component::Player, perform_action, send_messages, tick,
    BeforeActionNotification,
};

#[derive(Component)]
pub struct ActionQueue {
    /// The queue of actions to be performed.
    actions: VecDeque<Box<dyn Action>>,
    /// Actions that should be added to the beginning of the queue.
    to_add_front: Vec<Box<dyn Action>>,
    /// Actions that should be added to the end of the queue.
    to_add_back: Vec<Box<dyn Action>>,
}

impl ActionQueue {
    /// Determines if calling `update_queue` would modify the main actions queue.
    fn needs_update(&self) -> bool {
        !self.to_add_front.is_empty() || !self.to_add_back.is_empty()
    }

    /// Adds the actions from `to_add_front` and `to_add_back` to the main actions queue.
    fn update_queue(&mut self) {
        for action in self.to_add_front.drain(0..) {
            self.actions.push_front(action);
        }

        self.actions.extend(self.to_add_back.drain(0..));
    }
}

/// Queues an action for the provided entity
pub fn queue_action(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
        action_queue.to_add_back.push(action);
    } else {
        world.entity_mut(performing_entity).insert(ActionQueue {
            actions: VecDeque::new(),
            to_add_front: Vec::new(),
            to_add_back: vec![action],
        });
    }
}

/// Queues an action for the provided entity to perform before its other queued actions.
pub fn queue_action_first(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
        action_queue.to_add_front.push(action);
    } else {
        world.entity_mut(performing_entity).insert(ActionQueue {
            actions: VecDeque::new(),
            to_add_front: vec![action],
            to_add_back: Vec::new(),
        });
    }
}

/// Performs queued actions if all players have one queued.
pub fn try_perform_queued_actions(world: &mut World) {
    loop {
        debug!("Performing queued actions...");
        let mut entities_with_actions = Vec::new();
        for (entity, mut action_queue, _) in world
            .query::<(Entity, &mut ActionQueue, With<Player>)>()
            .iter_mut(world)
        {
            action_queue.update_queue();
            if action_queue.actions.is_empty() {
                // somebody doesn't have any action queued yet, so don't perform any
                debug!("{entity:?} has no queued actions, not performing any");
                return;
            }

            debug!("{entity:?} has a queued action");
            entities_with_actions.push(entity);
        }

        if entities_with_actions.is_empty() {
            return;
        }

        let mut results = Vec::new();
        for entity in entities_with_actions {
            if let Some(mut action) = determine_action_to_perform(entity, world) {
                let result = perform_action(world, entity, &mut action);
                results.push((entity, action, result));
            }
        }

        if results.iter().any(|(_, _, result)| result.should_tick) {
            tick(world);
        }

        for (entity, action, result) in results.into_iter() {
            send_messages(&result.messages, world);

            if !result.is_complete {
                //TODO interrupt action if something that would interrupt it has happened, like a hostile entity entering the performing entity's room
                queue_action_first(world, entity, action);
            }
        }
    }
}

fn determine_action_to_perform(entity: Entity, world: &mut World) -> Option<Box<dyn Action>> {
    let mut action_queue = world.get_mut::<ActionQueue>(entity)?;
    loop {
        let action = action_queue.actions.pop_front()?;

        //TODO before action notification handlers need to have some way to cancel the action, which probably means a separate validate notification

        action.send_before_notification(
            BeforeActionNotification {
                performing_entity: entity,
            },
            world,
        );

        action_queue = world.get_mut::<ActionQueue>(entity)?;
        if action_queue.needs_update() {
            // handlers for the before notification caused new actions to be queued, so make sure the correct action will be executed in case the
            // one we've got shouldn't be the first one anymore

            // `action` came from the front of the queue, so put it back
            action_queue.actions.push_front(action);
            // add newly queued actions on either side of it
            action_queue.update_queue();
        } else {
            // no handlers for the before notification caused new actions to be queued, so the one we've got is still the next one to execute
            return Some(action);
        };
    }
}
