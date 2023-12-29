use bevy_ecs::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    action::{ActionNotificationSender, AttackAction},
    notification::Notification,
    TickNotification,
};

use super::{queue_action, CombatState};

/// Makes an entity attack entities they are in combat with.
#[derive(Component)]
pub struct SelfDefenseBehavior;

/// Makes NPCs fight back.
pub fn attack_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut actions = Vec::new();
    let mut rng = rand::thread_rng();
    for (entity, _) in world.query::<(Entity, &SelfDefenseBehavior)>().iter(world) {
        if let Some(target) = CombatState::get_entities_in_combat_with(entity, world)
            .iter()
            .copied()
            .collect::<Vec<Entity>>()
            .choose(&mut rng)
        {
            let action: Box<AttackAction> = Box::new(AttackAction {
                target: *target,
                notification_sender: ActionNotificationSender::new(),
            });
            actions.push((entity, action));
        }
    }

    for (entity, action) in actions {
        queue_action(world, entity, action);
    }
}
