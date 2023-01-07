use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use log::debug;

use crate::{
    action::Action, component::Player, notification::NotificationType, send_messages, tick,
    InterruptedEntities,
};

const MAX_ACTION_QUEUE_LOOPS: u32 = 100000;
const MAX_ACTION_NOTIFICATION_LOOPS: u32 = 100000;

/// A notification sent to verify an action before it is performed.
#[derive(Debug)]
pub struct VerifyActionNotification {
    /// The entity that wants to perform the action.
    pub performing_entity: Entity,
}

impl NotificationType for VerifyActionNotification {}

/// A notification sent before an action is performed.
#[derive(Debug)]
pub struct BeforeActionNotification {
    /// The entity that will perform the action.
    pub performing_entity: Entity,
}

impl NotificationType for BeforeActionNotification {}

/// A notification sent after an action is performed.
#[derive(Debug)]
pub struct AfterActionNotification {
    /// The entity that performed the action.
    pub performing_entity: Entity,
    /// Whether the action is now complete.
    pub action_complete: bool,
    /// Whether the intended effects of the action actually ocurred.
    pub action_successful: bool,
}

impl NotificationType for AfterActionNotification {}

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
    let mut loops = 0;
    loop {
        if loops >= MAX_ACTION_QUEUE_LOOPS {
            panic!("players still have actions queued after {loops} loops")
        }
        loops += 1;

        debug!("Performing queued actions...");
        world.resource_mut::<InterruptedEntities>().0.clear();

        // first deal with actions that don't require a tick
        let players_with_action_queues = world
            .query::<(Entity, &mut ActionQueue, With<Player>)>()
            .iter_mut(world)
            .map(|(entity, mut action_queue, _)| {
                action_queue.update_queue();
                entity
            })
            .collect::<Vec<Entity>>();
        players_with_action_queues
            .into_iter()
            .for_each(|player| perform_tickless_actions(player, world));

        // now each player's action queue should either be empty, or have an action at the front that may require a tick to perform
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
            // new tickless actions may have been queued for this entity due to previously performed actions, so clear 'em out
            perform_tickless_actions(entity, world);

            if let Some(mut action) = determine_action_to_perform(entity, world, |_| true) {
                debug!("Entity {entity:?} is performing action {action:?}");
                let mut result = action.perform(entity, world);
                send_messages(&result.messages, world);
                action.send_after_notification(
                    AfterActionNotification {
                        performing_entity: entity,
                        action_complete: result.is_complete,
                        action_successful: result.was_successful,
                    },
                    world,
                );

                result.post_effects.drain(..).for_each(|f| f(world));

                results.push((entity, action, result));
            }
        }

        if results.iter().any(|(_, _, result)| result.should_tick) {
            tick(world);
        }

        for (entity, action, result) in results.into_iter() {
            if !result.is_complete {
                if world.resource::<InterruptedEntities>().0.contains(&entity) {
                    let interrupt_result = action.interrupt(entity, world);
                    send_messages(&interrupt_result.messages, world);
                    // cancel all other queued actions for this entity
                    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(entity) {
                        action_queue.actions.clear();
                    }
                    // the action was interrupted, so just drop it
                } else {
                    queue_action_first(world, entity, action);
                }
            }
        }
    }
}

/// Determines the next action for the provided entity to perform and sends pre-perform notifications for it, if the next action for the entity to perform passes the provided filter function.
fn determine_action_to_perform(
    entity: Entity,
    world: &mut World,
    filter_fn: fn(&Box<dyn Action>) -> bool,
) -> Option<Box<dyn Action>> {
    let mut loops = 0;
    loop {
        if loops >= MAX_ACTION_NOTIFICATION_LOOPS {
            panic!("action queue for entity {entity:?} still changing after {loops} loops")
        }
        loops += 1;

        let mut action_queue = world.get_mut::<ActionQueue>(entity)?;

        if action_queue
            .actions
            .front()
            .map_or(true, |action| !filter_fn(action))
        {
            return None;
        }

        let action = action_queue.actions.pop_front()?;

        action.send_before_notification(
            BeforeActionNotification {
                performing_entity: entity,
            },
            world,
        );

        let mut action_queue = world.get_mut::<ActionQueue>(entity)?;
        if action_queue.needs_update() {
            // handlers for the before notification caused new actions to be queued, so make sure the correct action will be executed in case the
            // one we've got shouldn't be the first one anymore

            // `action` came from the front of the queue, so put it back
            action_queue.actions.push_front(action);
            // add newly queued actions on either side of it
            action_queue.update_queue();
            continue;
        }

        let verify_result = action.send_verify_notification(
            VerifyActionNotification {
                performing_entity: entity,
            },
            world,
        );

        if verify_result.is_valid {
            return Some(action);
        } else {
            debug!("action {action:?} is invalid, canceling");
            send_messages(&verify_result.messages, world);
            // don't put the action back, it's invalid so just drop it
            continue;
        }
    }
}

/// Starting at the beginning of the provided entity's action queue, performs actions that don't require a tick until one that does require a tick,
/// or the end of the queue, is reached.
fn perform_tickless_actions(entity: Entity, world: &mut World) {
    loop {
        if let Some(mut action) =
            determine_action_to_perform(entity, world, |action| !action.may_require_tick())
        {
            debug!("Entity {entity:?} is performing action {action:?}");
            let result = action.perform(entity, world);
            send_messages(&result.messages, world);
            action.send_after_notification(
                AfterActionNotification {
                    performing_entity: entity,
                    action_complete: result.is_complete,
                    action_successful: result.was_successful,
                },
                world,
            );

            if !result.is_complete {
                queue_action_first(world, entity, action);
            }
        } else {
            return;
        }
    }
}
