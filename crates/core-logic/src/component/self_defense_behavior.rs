use bevy_ecs::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    action::{ActionNotificationSender, AttackAction},
    notification::Notification,
    TickNotification,
};

use super::{action_queue::has_any_queued_actions, queue_action, CombatRange, CombatState, Weapon};

/// Makes an entity attack entities they are in combat with.
#[derive(Component)]
pub struct SelfDefenseBehavior;

/// Makes NPCs fight back.
pub fn attack_on_tick(_: &Notification<TickNotification, ()>, world: &mut World) {
    let mut actions = Vec::new();
    let mut rng = rand::thread_rng();
    for (entity, _, combat_state) in world
        .query::<(Entity, &SelfDefenseBehavior, &CombatState)>()
        .iter(world)
    {
        if has_any_queued_actions(world, entity) {
            continue;
        }

        if let Some((weapon, _)) = Weapon::get_primary(entity, world) {
            let targets_in_range = combat_state
                .get_entities()
                .iter()
                .filter(|(_, range)| weapon.ranges.usable.contains(range))
                .collect::<Vec<(&Entity, &CombatRange)>>();

            if let Some((target, _)) = targets_in_range.choose(&mut rng) {
                // found someone in range
                let action: Box<AttackAction> = Box::new(AttackAction {
                    target: **target,
                    notification_sender: ActionNotificationSender::new(),
                });
                actions.push((entity, action));
            } else {
                // no one is in range, so try to move into range
                //TODO
            }
        }
    }

    for (entity, action) in actions {
        queue_action(world, entity, action);
    }
}
