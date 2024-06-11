use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use log::debug;

use crate::{
    action::Action, component::Player, notification::NotificationType, send_messages, tick,
    GameOptions, InterruptedEntities,
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

/// A notification sent after `perform` is called on an action.
#[derive(Debug)]
pub struct AfterActionPerformNotification {
    /// The entity that performed the action.
    pub performing_entity: Entity,
    /// Whether the action is now complete.
    pub action_complete: bool,
    /// Whether the intended effects of the action actually ocurred.
    pub action_successful: bool,
}

impl NotificationType for AfterActionPerformNotification {}

/// A notification sent after an action is done being performed, whether it completed successfully or was interrupted.
#[derive(Debug)]
pub struct ActionEndNotification {
    /// The entity that performed the action.
    pub performing_entity: Entity,
    /// Whether the action was interrupted.
    pub action_interrupted: bool,
}

impl NotificationType for ActionEndNotification {}

/// The state of a queued action.
#[derive(Clone, Copy)]
enum ActionState {
    /// `perform` has not yet been called on the action.
    NotStarted,
    /// `perform` has been called on the action at least once, but the action is not yet complete.
    InProgress,
}

#[derive(Component)]
pub struct ActionQueue {
    /// The queue of actions to be performed.
    actions: VecDeque<(Box<dyn Action>, ActionState)>,
    /// Actions that should be added to the beginning of the queue.
    to_add_front: Vec<(Box<dyn Action>, ActionState)>,
    /// Actions that should be added to the end of the queue.
    to_add_back: Vec<(Box<dyn Action>, ActionState)>,
}

impl ActionQueue {
    /// Creates an empty action queue.
    pub fn new() -> ActionQueue {
        ActionQueue {
            actions: VecDeque::new(),
            to_add_front: Vec::new(),
            to_add_back: Vec::new(),
        }
    }

    /// Determines whether the queue has any actions in it.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty() && self.to_add_front.is_empty() && self.to_add_back.is_empty()
    }

    /// Determines the total number of actions in the queue.
    pub fn number_of_actions(&self) -> usize {
        self.actions.len() + self.to_add_front.len() + self.to_add_back.len()
    }

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

    /// Clears all queued actions for the provided entity, interrupting any actions that are in progress.
    pub fn clear(world: &mut World, entity: Entity) {
        ActionQueue::cancel(|_| true, world, entity);
    }

    /// Cancels actions for the provided entity for which the provided function returns true, interrupting any actions that are in progress.
    pub fn cancel<F>(should_cancel: F, world: &mut World, entity: Entity)
    where
        F: Fn(&dyn Action) -> bool,
    {
        let mut actions_to_interrupt = Vec::new();

        if let Some(mut action_queue) = world.get_mut::<ActionQueue>(entity) {
            //TODO keep the 3 queues separate?
            action_queue.update_queue();
            /*
              This takes all the actions out of the queue, and then adds back the ones that shouldn't be canceled, which seems silly, but you can't
              remove from a list while iterating over it, and this is the only way I was able to get the ownership to work.
            */
            let mut actions_to_keep = VecDeque::new();
            for (action, state) in action_queue.actions.drain(0..) {
                if should_cancel(action.as_ref()) {
                    if let ActionState::InProgress = state {
                        actions_to_interrupt.push(action);
                    }
                } else {
                    actions_to_keep.push_back((action, state));
                }
            }
            action_queue.actions = actions_to_keep;
        }

        for action in actions_to_interrupt {
            interrupt_action(action.as_ref(), entity, world);
        }
    }

    /// Determines if the provided entity has any actions queued.
    pub fn has_any_queued_actions(world: &World, entity: Entity) -> bool {
        world
            .get::<ActionQueue>(entity)
            .map(|queue| !queue.is_empty())
            .unwrap_or(false)
    }

    /// Queues an action for the provided entity
    pub fn queue(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
        if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
            action_queue
                .to_add_back
                .push((action, ActionState::NotStarted));
        } else {
            world.entity_mut(performing_entity).insert(ActionQueue {
                actions: VecDeque::new(),
                to_add_front: Vec::new(),
                to_add_back: vec![(action, ActionState::NotStarted)],
            });
        }
    }

    /// Queues an action for the provided entity to perform before its other queued actions.
    pub fn queue_first(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
        queue_action_first_with_state(world, performing_entity, action, ActionState::NotStarted);
    }
}

/// Queues an action with the provided state for the provided entity to perform before its other queued actions.
fn queue_action_first_with_state(
    world: &mut World,
    performing_entity: Entity,
    action: Box<dyn Action>,
    state: ActionState,
) {
    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
        action_queue.to_add_front.push((action, state));
    } else {
        world.entity_mut(performing_entity).insert(ActionQueue {
            actions: VecDeque::new(),
            to_add_front: vec![(action, state)],
            to_add_back: Vec::new(),
        });
    }
}

/// Performs queued actions if all players have one queued.
/// Returns `true` if any actions were performed, `false` otherwise.
pub fn try_perform_queued_actions(world: &mut World) -> bool {
    let mut loops = 0;
    let mut any_actions_performed = false;
    loop {
        if loops >= MAX_ACTION_QUEUE_LOOPS {
            panic!("players still have actions queued after {loops} loops")
        }
        loops += 1;

        debug!("Performing queued actions...");
        world.resource_mut::<InterruptedEntities>().0.clear();

        // first deal with actions that don't require a tick
        let entities_with_action_queues = world
            .query::<(Entity, &mut ActionQueue)>()
            .iter_mut(world)
            .map(|(entity, mut action_queue)| {
                action_queue.update_queue();
                entity
            })
            .collect::<Vec<Entity>>();

        for entity in entities_with_action_queues.into_iter() {
            let any_tickless_actions_performed = perform_tickless_actions(entity, world);
            if any_tickless_actions_performed {
                any_actions_performed = true;
            }
        }

        // now each player's action queue should either be empty, or have an action at the front that may require a tick to perform
        let mut entities_with_actions = Vec::new();
        let afk_timeout = world.resource::<GameOptions>().afk_timeout;
        // players with actions
        for (entity, mut action_queue, player) in world
            .query::<(Entity, &mut ActionQueue, &Player)>()
            .iter_mut(world)
        {
            action_queue.update_queue();
            if !action_queue.actions.is_empty() {
                debug!("{entity:?} has a queued action");
                entities_with_actions.push(entity);
            } else if !player.is_afk(afk_timeout) {
                // somebody doesn't have any action queued yet, so don't perform any
                debug!("{entity:?} has no queued actions, not performing any");
                return any_actions_performed;
            }
        }

        if entities_with_actions.is_empty() {
            return any_actions_performed;
        }

        // non-players with actions
        for (entity, mut action_queue, _) in world
            .query::<(Entity, &mut ActionQueue, Without<Player>)>()
            .iter_mut(world)
        {
            action_queue.update_queue();
            if action_queue.actions.is_empty() {
                continue;
            }

            debug!("{entity:?} has a queued action");
            entities_with_actions.push(entity);
        }

        let mut results = Vec::new();
        for entity in entities_with_actions {
            // new tickless actions may have been queued for this entity due to previously performed actions, so clear 'em out
            perform_tickless_actions(entity, world);

            if let Some(mut action) = determine_action_to_perform(entity, world, |_| true) {
                debug!("Entity {entity:?} is performing action {action:?}");
                let mut result = action.perform(entity, world);
                any_actions_performed = true;
                send_messages(&result.messages, world);
                action.send_after_perform_notification(
                    AfterActionPerformNotification {
                        performing_entity: entity,
                        action_complete: result.is_complete,
                        action_successful: result.was_successful,
                    },
                    world,
                );

                if result.is_complete {
                    action.send_end_notification(
                        ActionEndNotification {
                            performing_entity: entity,
                            action_interrupted: false,
                        },
                        world,
                    );
                }

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
                    // cancel all other queued actions for this entity
                    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(entity) {
                        action_queue.actions.clear();
                    }
                    interrupt_action(action.as_ref(), entity, world);
                    // the action was interrupted, so just drop it
                } else {
                    queue_action_first_with_state(world, entity, action, ActionState::InProgress);
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
        action_queue.update_queue();

        if action_queue
            .actions
            .front()
            .map_or(true, |(action, _)| !filter_fn(action))
        {
            return None;
        }

        let (action, state) = action_queue.actions.pop_front()?;

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
            action_queue.actions.push_front((action, state));
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
///
/// Returns `true` if any actions were performed, `false` otherwise.
fn perform_tickless_actions(entity: Entity, world: &mut World) -> bool {
    let mut any_actions_performed = false;
    loop {
        if let Some(mut action) =
            determine_action_to_perform(entity, world, |action| !action.may_require_tick())
        {
            debug!("Entity {entity:?} is performing action {action:?}");
            let mut result = action.perform(entity, world);
            any_actions_performed = true;
            send_messages(&result.messages, world);
            action.send_after_perform_notification(
                AfterActionPerformNotification {
                    performing_entity: entity,
                    action_complete: result.is_complete,
                    action_successful: result.was_successful,
                },
                world,
            );

            if result.is_complete {
                action.send_end_notification(
                    ActionEndNotification {
                        performing_entity: entity,
                        action_interrupted: false,
                    },
                    world,
                );
            }

            result.post_effects.drain(..).for_each(|f| f(world));

            if !result.is_complete {
                ActionQueue::queue_first(world, entity, action);
            }
        } else {
            debug!("Done performing tickless actions for entity {entity:?}");
            return any_actions_performed;
        }
    }
}

/// Interrupts the provided action being performed by the provided entity.
fn interrupt_action(action: &dyn Action, entity: Entity, world: &mut World) {
    let interrupt_result = action.interrupt(entity, world);
    send_messages(&interrupt_result.messages, world);

    action.send_end_notification(
        ActionEndNotification {
            performing_entity: entity,
            action_interrupted: true,
        },
        world,
    );
}
