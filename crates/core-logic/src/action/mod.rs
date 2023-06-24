use bevy_ecs::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Mutex;

use crate::component::{
    ActionEndNotification, AfterActionPerformNotification, Container, Location,
};
use crate::notification::{
    Notification, NotificationHandlers, VerifyNotificationHandlers, VerifyResult,
};
use crate::{
    can_receive_messages, get_reference_name, send_message, BeforeActionNotification,
    MessageCategory, MessageDelay, VerifyActionNotification,
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

mod hold;
pub use hold::HoldAction;
pub use hold::HoldParser;

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

/// Registers notification handlers related to actions.
pub fn register_action_handlers(world: &mut World) {
    VerifyNotificationHandlers::add_handler(
        put::verify_source_and_destination_are_containers,
        world,
    );
    VerifyNotificationHandlers::add_handler(put::verify_item_in_source, world);
    VerifyNotificationHandlers::add_handler(put::prevent_put_item_inside_itself, world);
    VerifyNotificationHandlers::add_handler(put::prevent_put_non_item, world);

    NotificationHandlers::add_handler(r#move::look_after_move, world);

    NotificationHandlers::add_handler(wait::look_on_end_wait, world);

    NotificationHandlers::add_handler(sleep::look_on_end_sleep, world);

    VerifyNotificationHandlers::add_handler(wear::verify_has_item_to_wear, world);

    VerifyNotificationHandlers::add_handler(remove::prevent_remove_from_other_living_entity, world);

    VerifyNotificationHandlers::add_handler(hold::verify_has_item_to_hold, world);
    VerifyNotificationHandlers::add_handler(hold::verify_not_wearing_item_to_hold, world);
    NotificationHandlers::add_handler(hold::auto_unhold_on_hold, world);
}

/// A message caused by some other entity's action.
pub struct ThirdPersonMessage {
    /// The parts of the message.
    pub parts: Vec<MessagePart>,
    /// The category of the message.
    pub category: MessageCategory,
    /// The delay of the message.
    pub delay: MessageDelay,
}

impl ThirdPersonMessage {
    /// Creates a message with no parts.
    pub fn new(category: MessageCategory, delay: MessageDelay) -> ThirdPersonMessage {
        ThirdPersonMessage {
            parts: Vec::new(),
            category,
            delay,
        }
    }

    /// Adds a string to the message.
    pub fn add_string<T: Into<String>>(mut self, string: T) -> ThirdPersonMessage {
        self.parts.push(MessagePart::String(string.into()));

        self
    }

    /// Adds an entity name token to the message.
    pub fn add_entity_name(mut self, entity: Entity) -> ThirdPersonMessage {
        self.parts
            .push(MessagePart::Token(MessageToken::EntityName(entity)));

        self
    }

    /// Sends the message(s). No messages will be sent to `source_entity` if provided.
    pub fn send(
        self,
        source_entity: Option<Entity>,
        message_location: ThirdPersonMessageLocation,
        world: &World,
    ) {
        for (entity, message) in self.into_game_messages(source_entity, message_location, world) {
            send_message(world, entity, message);
        }
    }

    /// Builds messages for entities in `location`, excluding `source_entity` if provided.
    fn into_game_messages(
        self,
        source_entity: Option<Entity>,
        message_location: ThirdPersonMessageLocation,
        world: &World,
    ) -> HashMap<Entity, GameMessage> {
        let mut message_map = HashMap::new();

        let location = match message_location {
            ThirdPersonMessageLocation::SourceEntity => source_entity
                .and_then(|e| world.get::<Location>(e))
                .map(|loc| loc.id),
            ThirdPersonMessageLocation::Location(e) => Some(e),
        };

        if let Some(location) = location {
            if let Some(container) = world.get::<Container>(location) {
                for entity in &container.entities {
                    if Some(*entity) != source_entity && can_receive_messages(world, *entity) {
                        message_map.insert(*entity, self.to_game_message_for(*entity, world));
                    }
                }
            }
        }

        message_map
    }

    /// Builds a message for the provided entity.
    fn to_game_message_for(&self, pov_entity: Entity, world: &World) -> GameMessage {
        let mut resolved_parts = Vec::new();

        for part in &self.parts {
            match part {
                MessagePart::String(s) => resolved_parts.push(s.to_string()),
                MessagePart::Token(t) => resolved_parts.push(t.to_string(pov_entity, world)),
            }
        }

        GameMessage::Message {
            content: resolved_parts.join(""),
            category: self.category,
            delay: self.delay,
        }
    }
}

/// A part of a third-person message.
pub enum MessagePart {
    /// A literal string.
    String(String),
    /// A token to be interpolated for each message recipient.
    Token(MessageToken),
}

/// A token to be interpolated for each message recipient.
pub enum MessageToken {
    /// The name of an entity.
    EntityName(Entity),
}

/// The location to send a third-person message in.
pub enum ThirdPersonMessageLocation {
    /// The location of the entity that caused the message to be sent.
    SourceEntity,
    /// A specific location.
    Location(Entity),
}

impl MessageToken {
    /// Resolves the token to a string.
    fn to_string(&self, pov_entity: Entity, world: &World) -> String {
        match self {
            MessageToken::EntityName(e) => get_reference_name(*e, Some(pov_entity), world),
        }
    }
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
            },
        )
    }

    /// Adds messages to be sent to entities in `message_location`, excluding `source_entity` if provided.
    pub fn with_third_person_message(
        mut self,
        source_entity: Option<Entity>,
        message_location: ThirdPersonMessageLocation,
        third_person_message: ThirdPersonMessage,
        world: &World,
    ) -> ActionResultBuilder {
        for (entity, message) in
            third_person_message.into_game_messages(source_entity, message_location, world)
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
            .or_insert_with(Vec::new)
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
            },
        )
    }

    /// Adds messages to be sent to entities in `message_location`, excluding `source_entity` if provided.
    pub fn with_third_person_message(
        mut self,
        source_entity: Option<Entity>,
        message_location: ThirdPersonMessageLocation,
        third_person_message: ThirdPersonMessage,
        world: &World,
    ) -> ActionInterruptResultBuilder {
        for (entity, message) in
            third_person_message.into_game_messages(source_entity, message_location, world)
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
            .or_insert_with(Vec::new)
            .push(message);

        self
    }
}

pub trait Action: std::fmt::Debug + Send + Sync {
    /// Called when the provided entity should perform one tick of the action.
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult;

    /// Called when the action has been interrupted.
    fn interrupt(&self, performing_entity: Entity, world: &mut World) -> ActionInterruptResult;

    /// Returns whether the action might take game time to perform.
    /// TODO consider having 2 separate action traits, one for actions that might require a tick that takes in a mutable world, and one for actions that won't require a tick that takes in an immutable world
    fn may_require_tick(&self) -> bool;

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
