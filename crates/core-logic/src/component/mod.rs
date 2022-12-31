use bevy_ecs::prelude::*;

mod description;
pub use description::AttributeDescriber;
pub use description::AttributeDescription;
pub use description::AttributeDetailLevel;
pub use description::AttributeType;
pub use description::DescribeAttributes;
pub use description::Description;

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
pub use action_queue::queue_action;
pub use action_queue::queue_action_first;
pub use action_queue::try_perform_queued_actions;
pub use action_queue::ActionQueue;
pub use action_queue::AfterActionNotification;
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

mod hydration_factor;
pub use hydration_factor::HydrationFactor;

use crate::notification::NotificationHandlers;
use crate::notification::VerifyNotificationHandlers;

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

    NotificationHandlers::add_handler(vitals::reduce_vitals_on_tick, world);
    NotificationHandlers::add_handler(vitals::send_vitals_update_messages, world);
    NotificationHandlers::add_handler(vitals::interrupt_on_damage, world);
    NotificationHandlers::add_handler(vitals::kill_on_zero_health, world);

    NotificationHandlers::add_handler(respawner::look_after_respawn, world);

    NotificationHandlers::add_handler(calories::increase_satiety_on_eat, world);

    NotificationHandlers::add_handler(hydration_factor::increase_hydration_on_drink, world);
}
