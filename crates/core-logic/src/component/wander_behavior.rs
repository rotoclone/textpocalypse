use bevy_ecs::prelude::*;
use rand::{seq::SliceRandom, Rng};

use crate::{
    action::{ActionNotificationSender, MoveAction},
    notification::Notification,
    TickNotification,
};

use super::{ActionQueue, Container, Location};

/// Makes an entity wander around.
#[derive(Component)]
pub struct WanderBehavior {
    /// The chance the entity will move each tick.
    pub move_chance_per_tick: f32,
}

/// Makes wandering NPCs wander.
pub fn wander_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut actions = Vec::new();
    for (entity, wander_behavior) in world.query::<(Entity, &WanderBehavior)>().iter(world) {
        if ActionQueue::has_any_queued_actions(world, entity)
            || rand::thread_rng().gen::<f32>() > wander_behavior.move_chance_per_tick
        {
            continue;
        }

        if let Some(location) = world.get::<Location>(entity) {
            if let Some(container) = world.get::<Container>(location.id) {
                if let Some((_, connection)) = container
                    .get_connections(entity, world)
                    .choose(&mut rand::thread_rng())
                {
                    let action = Box::new(MoveAction {
                        direction: connection.direction,
                        notification_sender: ActionNotificationSender::new(),
                    });
                    actions.push((entity, action));
                }
            }
        }
    }

    for (entity, action) in actions {
        ActionQueue::queue(world, entity, action);
    }
}
