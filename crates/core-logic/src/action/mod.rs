use bevy_ecs::prelude::*;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Mutex;

use crate::component::{ActionEndNotification, AfterActionPerformNotification};
use crate::notification::{
    Notification, NotificationHandlers, VerifyNotificationHandlers, VerifyResult,
};
use crate::{
    combat_utils, BeforeActionNotification, DynamicMessage, DynamicMessageLocation,
    MessageCategory, MessageDelay, MessageTokens, VerifyActionNotification,
};
use crate::{GameMessage, World};

mod look;
pub use look::LookAction;
pub use look::LookParser;

mod r#move;
pub use r#move::MoveAction;
pub use r#move::MoveParser;

mod open;
pub use open::OpenAction;
pub use open::OpenParser;

mod help;
pub use help::HelpParser;

mod wait;
pub use wait::WaitParser;

mod inventory;
pub use inventory::InventoryParser;

mod put;
pub use put::PutAction;
pub use put::PutParser;

mod throw;
pub use throw::ThrowAction;
pub use throw::ThrowParser;

mod pour;
pub use pour::PourAction;
pub use pour::PourAmount;
pub use pour::PourParser;

mod wear;
pub use wear::WearAction;
pub use wear::WearParser;

mod remove;
pub use remove::RemoveAction;
pub use remove::RemoveParser;

mod equip;
pub use equip::EquipAction;
pub use equip::EquipParser;

mod vitals;
pub use vitals::VitalsParser;

mod stats;
pub use stats::StatsParser;

mod eat;
pub use eat::EatAction;
pub use eat::EatParser;

mod drink;
pub use drink::DrinkAction;
pub use drink::DrinkParser;

mod sleep;
pub use sleep::SleepAction;
pub use sleep::SleepParser;

mod say;
pub use say::SayAction;
pub use say::SayParser;

mod stop;
pub use stop::StopAction;
pub use stop::StopParser;

mod players;
pub use players::PlayersAction;
pub use players::PlayersParser;

mod worn;
pub use worn::WornAction;
pub use worn::WornParser;

mod attack;
pub use attack::AttackAction;
pub use attack::AttackParser;

mod change_range;
pub use change_range::ChangeRangeAction;
pub use change_range::ChangeRangeParser;
pub use change_range::RangeChangeDirection;

mod ranges;
pub use ranges::RangesAction;
pub use ranges::RangesParser;

mod spend_advacement_point;
pub use spend_advacement_point::SpendAdvancementPointParser;
pub use spend_advacement_point::SpendAttributePointAction;
pub use spend_advacement_point::SpendSkillPointAction;

/// Registers notification handlers related to actions.
pub fn register_action_handlers(world: &mut World) {
    VerifyNotificationHandlers::add_handler(
        put::verify_source_and_destination_are_containers,
        world,
    );
    VerifyNotificationHandlers::add_handler(put::verify_item_in_source, world);
    VerifyNotificationHandlers::add_handler(put::verify_item_not_in_destination, world);
    VerifyNotificationHandlers::add_handler(
        put::verify_source_not_owned_by_other_living_entity,
        world,
    );
    VerifyNotificationHandlers::add_handler(
        put::verify_destination_not_owned_by_other_living_entity,
        world,
    );
    VerifyNotificationHandlers::add_handler(put::prevent_put_item_inside_itself, world);
    VerifyNotificationHandlers::add_handler(put::prevent_put_non_item, world);

    NotificationHandlers::add_handler(throw::auto_equip_item_to_throw, world);
    VerifyNotificationHandlers::add_handler(throw::verify_wielding_item_to_throw, world);
    VerifyNotificationHandlers::add_handler(throw::verify_target_in_same_room, world);

    NotificationHandlers::add_handler(r#move::look_after_move, world);

    NotificationHandlers::add_handler(wait::look_on_end_wait, world);

    NotificationHandlers::add_handler(sleep::look_on_end_sleep, world);

    VerifyNotificationHandlers::add_handler(wear::verify_has_item_to_wear, world);

    VerifyNotificationHandlers::add_handler(remove::prevent_remove_from_other_living_entity, world);

    VerifyNotificationHandlers::add_handler(equip::verify_has_item_to_equip, world);
    VerifyNotificationHandlers::add_handler(equip::verify_not_wearing_item_to_equip, world);
    NotificationHandlers::add_handler(equip::auto_unequip_on_equip, world);

    VerifyNotificationHandlers::add_handler(
        combat_utils::verify_combat_action_valid::<AttackAction>,
        world,
    );
    NotificationHandlers::add_handler(combat_utils::equip_before_attack::<AttackAction>, world);

    NotificationHandlers::add_handler(combat_utils::cancel_attacks_when_exit_combat, world);

    VerifyNotificationHandlers::add_handler(change_range::verify_range_can_be_changed, world);
}

pub type PostEffectFn = Box<dyn FnOnce(&mut World)>;

/// The result of a single tick of an action being performed.
pub struct ActionResult {
    /// Any messages that should be sent.
    pub messages: HashMap<Entity, Vec<GameMessage>>,
    /// Whether a tick should happen due to the action being performed.
    pub should_tick: bool,
    /// Whether the action is now complete. If this is false, `perform` will be called on the action again.
    pub is_complete: bool,
    /// Whether the intended effects of the action actually ocurred.
    pub was_successful: bool,
    /// Functions to run after the action is complete and all its after action notification handlers have been invoked.
    pub post_effects: Vec<PostEffectFn>,
}

impl ActionResult {
    /// Creates an action result signifying that nothing of note occurred and the action was successful.
    pub fn none() -> ActionResult {
        ActionResult {
            messages: HashMap::new(),
            should_tick: false,
            is_complete: true,
            was_successful: true,
            post_effects: Vec::new(),
        }
    }

    /// Creates an action result with a single message for an entity, denoting that the action is complete and was successful.
    pub fn message(
        entity_id: Entity,
        message: String,
        category: MessageCategory,
        message_delay: MessageDelay,
        should_tick: bool,
    ) -> ActionResult {
        ActionResult {
            messages: [(
                entity_id,
                vec![GameMessage::Message {
                    content: message,
                    category,
                    delay: message_delay,
                    decorations: Vec::new(),
                }],
            )]
            .into(),
            should_tick,
            is_complete: true,
            was_successful: true,
            post_effects: Vec::new(),
        }
    }

    /// Creates an action result with a single error message for an entity, denoting that the action is complete and was not successful, and a tick should not happen.
    pub fn error(entity_id: Entity, message: String) -> ActionResult {
        ActionResult {
            messages: [(entity_id, vec![GameMessage::Error(message)])].into(),
            should_tick: false,
            is_complete: true,
            was_successful: false,
            post_effects: Vec::new(),
        }
    }

    /// Creates an `ActionResultBuilder`.
    pub fn builder() -> ActionResultBuilder {
        ActionResultBuilder {
            result: ActionResult::none(),
        }
    }
}

pub struct ActionResultBuilder {
    result: ActionResult,
}

impl ActionResultBuilder {
    /// Builds the `ActionResult`, denoting that the action has been completed and a tick should happen.
    pub fn build_complete_should_tick(mut self, was_successful: bool) -> ActionResult {
        self.result.should_tick = true;
        self.result.is_complete = true;
        self.result.was_successful = was_successful;
        self.result
    }

    /// Builds the `ActionResult`, denoting that the action has been completed and a tick should not happen.
    pub fn build_complete_no_tick(mut self, was_successful: bool) -> ActionResult {
        self.result.should_tick = false;
        self.result.is_complete = true;
        self.result.was_successful = was_successful;
        self.result
    }

    /// Builds the `ActionResult`, denoting that the action has not been completed.
    pub fn build_incomplete(mut self, was_successful: bool) -> ActionResult {
        self.result.should_tick = true;
        self.result.is_complete = false;
        self.result.was_successful = was_successful;
        self.result
    }

    /// Adds a message to be sent to an entity.
    pub fn with_message(
        self,
        entity_id: Entity,
        message: String,
        category: MessageCategory,
        message_delay: MessageDelay,
    ) -> ActionResultBuilder {
        self.with_game_message(
            entity_id,
            GameMessage::Message {
                content: message,
                category,
                delay: message_delay,
                decorations: Vec::new(),
            },
        )
    }

    /// Adds messages to be sent to entities in `message_location`, generated by `source_entity`.
    pub fn with_dynamic_message<T: MessageTokens>(
        mut self,
        source_entity: Option<Entity>,
        message_location: DynamicMessageLocation,
        dynamic_message: DynamicMessage<T>,
        world: &World,
    ) -> ActionResultBuilder {
        for (entity, message) in dynamic_message
            .into_game_messages(source_entity, message_location, world)
            .expect("message interpolation should not fail")
        {
            self = self.with_game_message(entity, message);
        }

        self
    }

    /// Adds an error message to be sent to an entity.
    pub fn with_error(self, entity_id: Entity, message: String) -> ActionResultBuilder {
        self.with_game_message(entity_id, GameMessage::Error(message))
    }

    /// Adds a `GameMessage` to be sent to an entity.
    pub fn with_game_message(
        mut self,
        entity_id: Entity,
        message: GameMessage,
    ) -> ActionResultBuilder {
        self.result
            .messages
            .entry(entity_id)
            .or_default()
            .push(message);

        self
    }

    /// Adds a post-effect to be executed after all the action's after notification handlers have been invoked.
    pub fn with_post_effect(mut self, effect: PostEffectFn) -> ActionResultBuilder {
        self.result.post_effects.push(effect);

        self
    }
}

/// The result of an action being interrupted.
#[derive(Debug)]
pub struct ActionInterruptResult {
    /// Any messages that should be sent.
    pub messages: HashMap<Entity, Vec<GameMessage>>,
}

impl ActionInterruptResult {
    /// Creates an action interrupt result with no messages.
    pub fn none() -> ActionInterruptResult {
        ActionInterruptResult {
            messages: HashMap::new(),
        }
    }

    /// Creates an action interrupt result with a single message for an entity.
    pub fn message(
        entity_id: Entity,
        message: String,
        category: MessageCategory,
        message_delay: MessageDelay,
    ) -> ActionInterruptResult {
        ActionInterruptResult {
            messages: [(
                entity_id,
                vec![GameMessage::Message {
                    content: message,
                    category,
                    delay: message_delay,
                    decorations: Vec::new(),
                }],
            )]
            .into(),
        }
    }

    /// Creates an action interrupt result with a single error message for an entity.
    pub fn error(entity_id: Entity, message: String) -> ActionInterruptResult {
        ActionInterruptResult {
            messages: [(entity_id, vec![GameMessage::Error(message)])].into(),
        }
    }

    /// Creates an `ActionInterruptResultBuilder`.
    pub fn builder() -> ActionInterruptResultBuilder {
        ActionInterruptResultBuilder {
            result: ActionInterruptResult::none(),
        }
    }
}

pub struct ActionInterruptResultBuilder {
    result: ActionInterruptResult,
}

impl ActionInterruptResultBuilder {
    /// Builds the `ActionInterruptResult`.
    pub fn build(self) -> ActionInterruptResult {
        self.result
    }

    /// Adds a message to be sent to an entity.
    pub fn with_message(
        self,
        entity_id: Entity,
        message: String,
        category: MessageCategory,
        message_delay: MessageDelay,
    ) -> ActionInterruptResultBuilder {
        self.with_game_message(
            entity_id,
            GameMessage::Message {
                content: message,
                category,
                delay: message_delay,
                decorations: Vec::new(),
            },
        )
    }

    /// Adds messages to be sent to entities in `message_location`, excluding `source_entity` if provided.
    pub fn with_dynamic_message<T: MessageTokens>(
        mut self,
        source_entity: Option<Entity>,
        message_location: DynamicMessageLocation,
        dynamic_message: DynamicMessage<T>,
        world: &World,
    ) -> ActionInterruptResultBuilder {
        for (entity, message) in dynamic_message
            .into_game_messages(source_entity, message_location, world)
            .expect("message interpolation should not fail")
        {
            self = self.with_game_message(entity, message);
        }

        self
    }

    /// Adds an error message to be sent to an entity.
    pub fn with_error(self, entity_id: Entity, message: String) -> ActionInterruptResultBuilder {
        self.with_game_message(entity_id, GameMessage::Error(message))
    }

    /// Adds a `GameMessage` to be sent to an entity.
    fn with_game_message(
        mut self,
        entity_id: Entity,
        message: GameMessage,
    ) -> ActionInterruptResultBuilder {
        self.result
            .messages
            .entry(entity_id)
            .or_default()
            .push(message);

        self
    }
}

/// Represents a tag that describes an action.
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ActionTag {
    /// For actions that have to do with combat.
    Combat,
    /// A non-standard tag.
    Custom(String),
}

pub trait Action: std::fmt::Debug + Send + Sync {
    /// Called when the provided entity should perform one tick of the action.
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult;

    /// Called when the action has been interrupted.
    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult;

    /// Returns whether the action might take game time to perform.
    /// TODO consider having 2 separate action traits, one for actions that might require a tick that takes in a mutable world, and one for actions that won't require a tick that takes in an immutable world
    fn may_require_tick(&self) -> bool;

    /// Returns the tags of this action, so it can be identified.
    fn get_tags(&self) -> HashSet<ActionTag>;

    /// Sends a notification that this action is about to be performed, if one hasn't already been sent for this action.
    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    );

    /// Sends a notification to verify that this action is valid.
    fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        world: &mut World,
    ) -> VerifyResult;

    /// Sends a notification that `perform` was just called on this action.
    fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        world: &mut World,
    );

    /// Sends a notification that this action is done being performed.
    fn send_end_notification(&self, notification_type: ActionEndNotification, world: &mut World);
}

/// Sends notifications about actions.
#[derive(Debug)]
pub struct ActionNotificationSender<C: Send + Sync> {
    before_notification_sent: Mutex<bool>,
    _c: PhantomData<fn(C)>,
}

impl<C: Send + Sync + 'static> ActionNotificationSender<C> {
    /// Creates a new `ActionNotificationSender`.
    pub fn new() -> Self {
        Self {
            before_notification_sent: Mutex::new(false),
            _c: PhantomData,
        }
    }

    /// Sends a notification that an action is about to be performed, if one hasn't already been sent by this sender.
    pub fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        contents: &C,
        world: &mut World,
    ) {
        if !*self.before_notification_sent.lock().unwrap() {
            *self.before_notification_sent.lock().unwrap() = true;
            Notification {
                notification_type,
                contents,
            }
            .send(world);
        }
    }

    /// Sends a notification to verify that an action is valid.
    pub fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        contents: &C,
        world: &World,
    ) -> VerifyResult {
        Notification {
            notification_type,
            contents,
        }
        .verify(world)
    }

    /// Sends a notification that `perform` was called on an action.
    pub fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        contents: &C,
        world: &mut World,
    ) {
        Notification {
            notification_type,
            contents,
        }
        .send(world);
    }

    /// Sends a notification that an action is done being performed.
    pub fn send_end_notification(
        &self,
        notification_type: ActionEndNotification,
        contents: &C,
        world: &mut World,
    ) {
        Notification {
            notification_type,
            contents,
        }
        .send(world);
    }
}
