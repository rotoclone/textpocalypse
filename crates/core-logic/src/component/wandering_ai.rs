use bevy_ecs::prelude::*;
use rand::{seq::SliceRandom, Rng};

use crate::{
    action::{ActionNotificationSender, MoveAction},
    notification::Notification,
    TickNotification,
};

use super::{queue_action, Container, Location};

/// Makes an entity wander around.
#[derive(Component)]
pub struct WanderingAi {
    /// The chance the entity will move each tick.
    pub move_chance_per_tick: f32,
}

/// Makes wandering NPCs wander.
pub fn wander_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut actions = Vec::new();
    for (entity, wandering_ai) in world.query::<(Entity, &WanderingAi)>().iter(world) {
        if rand::thread_rng().gen::<f32>() <= wandering_ai.move_chance_per_tick {
            if let Some(location) = world.get::<Location>(entity) {
                if let Some(container) = world.get::<Container>(location.id) {
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
    }

    for (entity, action) in actions {
        queue_action(world, entity, action);
    }
}
