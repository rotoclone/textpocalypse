use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

use bevy_ecs::prelude::*;

use crate::{action::MoveAction, component::BeforeActionNotification, GameMessage};

/// Trait for types that can be sent as notifications.
pub trait Notification: Debug + Send + Sync + Sized {
    /* TODO
    fn send(&self, world: &mut World) {
        if let Some(handlers) = world.get_resource::<NotificationHandlers<Self>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .cloned()
                .collect::<Vec<HandleFn<Self>>>();

            for handle_fn in handle_fns {
                handle_fn(self, world);
            }
        }
    }
    */

    /* TODO
    fn verify(&self, world: &World) -> VerifyResult {
        if let Some(handlers) = world.get_resource::<VerifyNotificationHandlers<Self>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .cloned()
                .collect::<Vec<HandleVerifyFn<Self>>>();

            for handle_fn in handle_fns {
                let response = handle_fn(self, world);
                if !response.is_valid {
                    return response;
                }
            }
        }

        VerifyResult::valid()
    }
    */
}

//TODO remove
fn send_notification_specific(
    notification: BeforeActionNotification<MoveAction>,
    world: &mut World,
) {
    if let Some(handlers) =
        world.get_resource::<NotificationHandlers<BeforeActionNotification<MoveAction>>>()
    {
        let handle_fns = handlers
            .handlers
            .values()
            .cloned()
            .collect::<Vec<HandleFn<BeforeActionNotification<MoveAction>>>>();

        for handle_fn in handle_fns {
            handle_fn(&notification, world);
        }
    }
}

pub fn send_notification<N: Notification + 'static>(notification: N, world: &mut World) {
    if let Some(handlers) = world.get_resource::<NotificationHandlers<N>>() {
        let handle_fns = handlers
            .handlers
            .values()
            .cloned()
            .collect::<Vec<HandleFn<N>>>();

        for handle_fn in handle_fns {
            handle_fn(&notification, world);
        }
    }
}

pub fn send_verify_notification<N: Notification + 'static>(
    notification: N,
    world: &World,
) -> VerifyResult {
    if let Some(handlers) = world.get_resource::<VerifyNotificationHandlers<N>>() {
        let handle_fns = handlers
            .handlers
            .values()
            .cloned()
            .collect::<Vec<HandleVerifyFn<N>>>();

        for handle_fn in handle_fns {
            let response = handle_fn(&notification, world);
            if !response.is_valid {
                return response;
            }
        }
    }

    VerifyResult::valid()
}

/// An identifier for a registered notification handler.
///
/// This is only unique to the notification type + resource type combo.
/// For example, the first handler registered for `BeforeActionNotification<MoveAction>` and the first one registered for
/// `BeforeActionNotification<LookAction>` will both have the same internal value, just different associated types.
pub struct NotificationHandlerId<N: Notification, R> {
    value: u64,
    _n: PhantomData<fn(N)>,
    _r: PhantomData<fn(R)>,
}

// need to manually implement traits due to https://github.com/rust-lang/rust/issues/26925
impl<N: Notification, R> Clone for NotificationHandlerId<N, R> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            _n: PhantomData,
            _r: PhantomData,
        }
    }
}

impl<N: Notification, R> Copy for NotificationHandlerId<N, R> {}

impl<N: Notification, R> PartialEq for NotificationHandlerId<N, R> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<N: Notification, R> Eq for NotificationHandlerId<N, R> {}

impl<N: Notification, R> Hash for NotificationHandlerId<N, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<N: Notification, R> NotificationHandlerId<N, R> {
    /// Creates a new notification handler ID with the minimum starting value.
    fn new() -> NotificationHandlerId<N, R> {
        NotificationHandlerId {
            value: 0,
            _n: PhantomData,
            _r: PhantomData,
        }
    }

    /// Increments this notification handler ID's value.
    fn next(mut self) -> NotificationHandlerId<N, R> {
        self.value += 1;
        self
    }
}

/// Signature of a function to handle notifications.
type HandleFn<N: Notification> = fn(&N, &mut World);

/// Type of the notification handler ID for regular notifications.
type HandlerId<N> = NotificationHandlerId<N, NotificationHandlers<N>>;

/// The set of notification handlers for a single notification type.
#[derive(Resource)]
pub struct NotificationHandlers<N: Notification> {
    /// The ID to be assigned to the next registered handler.
    next_id: HandlerId<N>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<HandlerId<N>, HandleFn<N>>,
}

impl<N: Notification + 'static> NotificationHandlers<N> {
    /// Creates a new, empty set of handlers.
    fn new() -> NotificationHandlers<N> {
        NotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    /// Adds the provided handler to this set of handlers and returns its assigned ID.
    fn add(&mut self, handle_fn: HandleFn<N>) -> HandlerId<N> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(handler: HandleFn<N>, world: &mut World) -> HandlerId<N> {
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<N>>() {
            return handlers.add(handler);
        }

        let mut handlers = NotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: HandlerId<N>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<N>>() {
            handlers.handlers.remove(&id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<NotificationHandlers<N>>();
        }
    }
}

/// Result of verifying the contents of a notification.
pub struct VerifyResult {
    /// Whether the notification contents are valid.
    pub is_valid: bool,
    /// Any messages to send to relevant entities explaining why the notification was invalid.
    pub messages: HashMap<Entity, Vec<GameMessage>>,
}

impl VerifyResult {
    /// Creates a result denoting that the notification contents are valid.
    pub fn valid() -> VerifyResult {
        VerifyResult {
            is_valid: true,
            messages: HashMap::new(),
        }
    }

    /// Creates a result denoting that the notification contents are invalid with a single message for an entity.
    pub fn invalid(entity: Entity, message: GameMessage) -> VerifyResult {
        Self::invalid_with_messages([(entity, vec![message])].into())
    }

    /// Creates a result denoting that the notification contents are invalid.
    pub fn invalid_with_messages(messages: HashMap<Entity, Vec<GameMessage>>) -> VerifyResult {
        VerifyResult {
            is_valid: false,
            messages,
        }
    }
}

/// Signature of a function to handle verify notifications.
/// TODO if verify notifications will all have their own `VerifySomething` notification type, maybe just make handlers generic over their return type instead of having a special handler type that specifically returns `VerifyResult`
type HandleVerifyFn<N> = fn(&N, &World) -> VerifyResult;

/// Type of the notification handler ID for verify notifications.
type VerifyHandlerId<N> = NotificationHandlerId<N, VerifyNotificationHandlers<N>>;

// TODO consider removing this duplication

/// The set of verify notification handlers for a single notification type.
#[derive(Resource)]
pub struct VerifyNotificationHandlers<N: Notification> {
    /// The ID to be assigned to the next registered handler.
    next_id: VerifyHandlerId<N>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<VerifyHandlerId<N>, HandleVerifyFn<N>>,
}

impl<N: Notification + 'static> VerifyNotificationHandlers<N> {
    /// Creates a new, empty set of handlers.
    fn new() -> VerifyNotificationHandlers<N> {
        VerifyNotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    /// Adds the provided handler to this set of handlers and returns its assigned ID.
    fn add(&mut self, handle_fn: HandleVerifyFn<N>) -> VerifyHandlerId<N> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(handler: HandleVerifyFn<N>, world: &mut World) -> VerifyHandlerId<N> {
        if let Some(mut handlers) = world.get_resource_mut::<VerifyNotificationHandlers<N>>() {
            return handlers.add(handler);
        }

        let mut handlers = VerifyNotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: VerifyHandlerId<N>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(mut handlers) = world.get_resource_mut::<VerifyNotificationHandlers<N>>() {
            handlers.handlers.remove(&id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<VerifyNotificationHandlers<N>>();
        }
    }
}
