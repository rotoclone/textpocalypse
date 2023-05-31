use bevy_ecs::prelude::*;
use rand::{seq::SliceRandom, Rng};

use crate::{
    action::{ActionNotificationSender, MoveAction},
    notification::Notification,
    TickNotification,
};

use super::{
    location::{get_concrete_location, ConcreteLocation},
    queue_action,
};

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
        if rand::thread_rng().gen::<f32>() <= wander_behavior.move_chance_per_tick {
            if let Some(ConcreteLocation::Container(_, container)) =
                get_concrete_location(entity, world)
            {
                if let Some((_, connection)) = container
                    .get_connections(world)
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
        queue_action(world, entity, action);
    }
}
