use bevy_ecs::prelude::*;

mod description;
pub use description::AttributeDescriber;
pub use description::AttributeDescription;
pub use description::AttributeDetailLevel;
pub use description::AttributeType;
pub use description::DescribeAttributes;
pub use description::Description;
pub use description::Pronouns;

mod location;
pub use location::Location;

mod room;
pub use room::Room;

mod connection;
pub use connection::Connection;

mod open_state;
pub use open_state::OpenState;

mod keyed_lock;
pub use keyed_lock::KeyId;
pub use keyed_lock::KeyedLock;

mod custom_input_parser;
pub use custom_input_parser::CustomInputParser;
pub use custom_input_parser::ParseCustomInput;

mod player;
pub use player::Player;
pub use player::PlayerId;

mod action_queue;
pub use action_queue::clear_action_queue;
pub use action_queue::queue_action;
pub use action_queue::queue_action_first;
pub use action_queue::try_perform_queued_actions;
pub use action_queue::ActionEndNotification;
pub use action_queue::ActionQueue;
pub use action_queue::AfterActionPerformNotification;
pub use action_queue::BeforeActionNotification;
pub use action_queue::VerifyActionNotification;

mod container;
pub use container::Container;

mod fluid_container;
pub use fluid_container::FluidContainer;

mod volume;
pub use volume::Volume;

mod weight;
pub use weight::Weight;

mod density;
pub use density::Density;

mod vitals;
pub use vitals::Vitals;

mod respawner;
pub use respawner::Respawner;

mod edible;
pub use edible::Edible;

mod calories;
pub use calories::Calories;

mod fluid;
pub use fluid::Fluid;
pub use fluid::FluidType;
pub use fluid::FluidTypeAmount;

mod sleep_state;
pub use sleep_state::is_asleep;
pub use sleep_state::SleepState;

mod wander_behavior;
pub use wander_behavior::WanderBehavior;

mod greet_behavior;
pub use greet_behavior::GreetBehavior;

mod item;
pub use item::get_hands_to_equip;
pub use item::Item;

mod stats;
pub use stats::Attribute;
pub use stats::Attributes;
pub use stats::Skill;
pub use stats::Skills;
pub use stats::Stat;
pub use stats::Stats;

mod wearable;
pub use wearable::Wearable;

mod worn_items;
pub use worn_items::RemoveError;
pub use worn_items::WearError;
pub use worn_items::WornItems;

mod equipped_items;
pub use equipped_items::EquipError;
pub use equipped_items::EquippedItems;
pub use equipped_items::UnequipError;

use crate::notification::Notification;
use crate::notification::NotificationHandlers;
use crate::notification::VerifyNotificationHandlers;
use crate::DeathNotification;

/// Registers notification handlers related to components.
pub fn register_component_handlers(world: &mut World) {
    NotificationHandlers::add_handler(open_state::auto_open_connections, world);
    VerifyNotificationHandlers::add_handler(
        open_state::prevent_moving_through_closed_connections,
        world,
    );

    NotificationHandlers::add_handler(keyed_lock::auto_unlock_keyed_locks, world);
    VerifyNotificationHandlers::add_handler(keyed_lock::prevent_opening_locked_keyed_locks, world);

    VerifyNotificationHandlers::add_handler(container::limit_container_contents, world);

    NotificationHandlers::add_handler(vitals::change_vitals_on_tick, world);
    NotificationHandlers::add_handler(vitals::send_vitals_update_messages, world);
    NotificationHandlers::add_handler(vitals::interrupt_on_damage, world);
    NotificationHandlers::add_handler(vitals::kill_on_zero_health, world);
    NotificationHandlers::add_handler(vitals::sleep_on_zero_energy, world);

    NotificationHandlers::add_handler(respawner::look_after_respawn, world);

    NotificationHandlers::add_handler(calories::increase_satiety_on_eat, world);

    VerifyNotificationHandlers::add_handler(fluid_container::verify_source_container, world);
    VerifyNotificationHandlers::add_handler(fluid_container::limit_fluid_container_contents, world);

    NotificationHandlers::add_handler(wander_behavior::wander_on_tick, world);
    NotificationHandlers::add_handler(remove_on_death::<WanderBehavior>, world);

    NotificationHandlers::add_handler(greet_behavior::greet_new_entities, world);
    NotificationHandlers::add_handler(remove_on_death::<GreetBehavior>, world);

    VerifyNotificationHandlers::add_handler(sleep_state::prevent_look_while_asleep, world);
    VerifyNotificationHandlers::add_handler(sleep_state::prevent_say_while_asleep, world);

    NotificationHandlers::add_handler(worn_items::auto_remove_on_put, world);
    VerifyNotificationHandlers::add_handler(worn_items::verify_not_wearing_item_to_put, world);

    NotificationHandlers::add_handler(equipped_items::unequip_on_put, world);
    NotificationHandlers::add_handler(equipped_items::unequip_on_wear, world);
}

/// Removes a component from an entity when it dies.
fn remove_on_death<T: Bundle>(
    notification: &Notification<DeathNotification, ()>,
    world: &mut World,
) {
    world
        .entity_mut(notification.notification_type.entity)
        .remove::<T>();
}
