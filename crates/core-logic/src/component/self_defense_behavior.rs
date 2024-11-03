use bevy_ecs::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    action::{
        Action, ActionNotificationSender, AttackAction, ChangeRangeAction, RangeChangeDirection,
    },
    notification::Notification,
    ChosenWeapon, TickNotification,
};

use super::{ActionQueue, CombatRange, CombatState, Weapon};

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
        if ActionQueue::has_any_queued_actions(world, entity) {
            continue;
        }

        if let Some((weapon, weapon_entity)) = Weapon::get_primary(entity, world) {
            let targets_in_range = combat_state
                .get_entities()
                .iter()
                .filter(|(_, range)| weapon.ranges.usable.contains(range))
                .collect::<Vec<(&Entity, &CombatRange)>>();

            if let Some((target, _)) = targets_in_range.choose(&mut rng) {
                // found someone in range
                let action: Box<dyn Action> = Box::new(AttackAction {
                    target: **target,
                    weapon: ChosenWeapon::Entity(weapon_entity),
                    notification_sender: ActionNotificationSender::new(),
                });
                actions.push((entity, action));
            } else {
                // no one is in range, so try to move into range of the combatant closest to being in range
                if let Some((combatant, range_diff)) = combat_state
                    .get_entities()
                    .iter()
                    .map(|(combatant, range)| (combatant, weapon.get_usable_range_diff(*range)))
                    .min_by(|(_, diff_1), (_, diff_2)| diff_1.abs().cmp(&diff_2.abs()))
                {
                    let direction = if range_diff < 0 {
                        RangeChangeDirection::Increase
                    } else {
                        RangeChangeDirection::Decrease
                    };
                    let action: Box<dyn Action> = Box::new(ChangeRangeAction {
                        target: *combatant,
                        direction,
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
