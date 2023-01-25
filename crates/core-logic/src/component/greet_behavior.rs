use bevy_ecs::prelude::*;

use crate::action::{ActionNotificationSender, MoveAction, SayAction};

use super::{queue_action, AfterActionPerformNotification, Container, Location};

/// Makes an entity greet entities that enter its location.
#[derive(Component)]
pub struct GreetBehavior {
    /// What the entity will say as a greeting.
    pub greeting: String,
}

/// Makes greeting NPCs greet.
pub fn greet_new_entities(
    notification: &AfterActionPerformNotification<MoveAction>,
    world: &mut World,
) {
    if !notification.action_complete || !notification.action_successful {
        return;
    }

    //TODO don't greet multiple times if multiple entities enter in the same tick

    let mut actions = Vec::new();
    for (entity, greet_behavior) in world.query::<(Entity, &GreetBehavior)>().iter(world) {
        if entity == notification.performing_entity {
            // don't need to greet yourself
            continue;
        }
        if let Some(location) = world.get::<Location>(entity) {
            if let Some(container) = world.get::<Container>(location.id) {
                if container.entities.contains(&notification.performing_entity) {
                    // the move action was successful, and the entity that performed it is standing here, so they must have just arrived
                    actions.push((
                        entity,
                        Box::new(SayAction {
                            text: greet_behavior.greeting.clone(),
                            notification_sender: ActionNotificationSender::new(),
                        }),
                    ));
                }
            }
        }
    }

    for (entity, action) in actions {
        queue_action(world, entity, action);
    }
}
