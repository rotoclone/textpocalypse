use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

use bevy_ecs::prelude::*;

use crate::GameMessage;

/// Trait for types that represent a category of notifications.
pub trait NotificationType: Debug + Send + Sync {}

/// A notification.
#[derive(Debug)]
pub struct Notification<'c, T: NotificationType, Contents> {
    /// The type of the notification.
    pub notification_type: T,
    /// The contents of the notification.
    pub contents: &'c Contents,
}

impl<'c, T: NotificationType + 'static, C: Send + Sync + 'static> Notification<'c, T, C> {
    /// Sends this notification to all the handlers registered for it.
    pub fn send(&self, world: &mut World) {
        if let Some(handlers) = world.get_resource::<NotificationHandlers<T, C>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .cloned()
                .collect::<Vec<HandleFn<T, C>>>();

            for handle_fn in handle_fns {
                handle_fn(self, world);
            }
        }
    }

    /// Sends this notification to all the verify handlers registered for it.
    pub fn verify(&self, world: &World) -> VerifyResult {
        if let Some(handlers) = world.get_resource::<VerifyNotificationHandlers<T, C>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .cloned()
                .collect::<Vec<HandleVerifyFn<T, C>>>();

            for handle_fn in handle_fns {
                let response = handle_fn(self, world);
                if !response.is_valid {
                    return response;
                }
            }
        }

        VerifyResult::valid()
    }
}

/// An identifier for a registered notification handler.
///
/// This is only unique to the notification type + contents + resource type type combo.
/// For example, the first handler registered for `BeforeActionNotification` and `MoveAction` and the first one registered for
/// `BeforeActionNotification` and `LookAction` will both have the same internal value, just different associated types.
pub struct NotificationHandlerId<T: NotificationType, C: Send + Sync, R> {
    value: u64,
    _t: PhantomData<fn(T)>,
    _c: PhantomData<fn(C)>,
    _r: PhantomData<fn(R)>,
}

// need to manually implement traits due to https://github.com/rust-lang/rust/issues/26925
impl<T: NotificationType, C: Send + Sync, R> Clone for NotificationHandlerId<T, C, R> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            _t: PhantomData,
            _c: PhantomData,
            _r: PhantomData,
        }
    }
}

impl<T: NotificationType, C: Send + Sync, R> Copy for NotificationHandlerId<T, C, R> {}

impl<T: NotificationType, C: Send + Sync, R> PartialEq for NotificationHandlerId<T, C, R> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: NotificationType, C: Send + Sync, R> Eq for NotificationHandlerId<T, C, R> {}

impl<T: NotificationType, C: Send + Sync, R> Hash for NotificationHandlerId<T, C, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: NotificationType, C: Send + Sync, R> NotificationHandlerId<T, C, R> {
    /// Creates a new notification handler ID with the minimum starting value.
    fn new() -> NotificationHandlerId<T, C, R> {
        NotificationHandlerId {
            value: 0,
            _t: PhantomData,
            _c: PhantomData,
            _r: PhantomData,
        }
    }

    /// Increments this notification handler ID's value.
    fn next(mut self) -> NotificationHandlerId<T, C, R> {
        self.value += 1;
        self
    }
}

/// Signature of a function to handle notifications.
type HandleFn<T, Contents> = fn(&Notification<T, Contents>, &mut World);

/// Type of the notification handler ID for regular notifications.
type HandlerId<T, C> = NotificationHandlerId<T, C, NotificationHandlers<T, C>>;

/// The set of notification handlers for a single notification type and contents type combination.
#[derive(Resource)]
pub struct NotificationHandlers<T: NotificationType, C: Send + Sync> {
    /// The ID to be assigned to the next registered handler.
    next_id: NotificationHandlerId<T, C, NotificationHandlers<T, C>>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<HandlerId<T, C>, HandleFn<T, C>>,
}

impl<T: NotificationType + 'static, C: Send + Sync + 'static> NotificationHandlers<T, C> {
    /// Creates a new, empty set of handlers.
    fn new() -> NotificationHandlers<T, C> {
        NotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    /// Adds the provided handler to this set of handlers and returns its assigned ID.
    fn add(&mut self, handle_fn: HandleFn<T, C>) -> HandlerId<T, C> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(handler: HandleFn<T, C>, world: &mut World) -> HandlerId<T, C> {
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            return handlers.add(handler);
        }

        let mut handlers = NotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: HandlerId<T, C>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            handlers.handlers.remove(&id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<NotificationHandlers<T, C>>();
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

    /// Creates a result denoting that the notification contents are invalid.
    pub fn invalid(messages: HashMap<Entity, Vec<GameMessage>>) -> VerifyResult {
        VerifyResult {
            is_valid: false,
            messages,
        }
    }
}

/// Signature of a function to handle verify notifications.
/// TODO if verify notifications will all have their own `VerifySomething` notification type, maybe just make handlers generic over their return type instead of having a special handler type that specifically returns `VerifyResult`
type HandleVerifyFn<T, Contents> = fn(&Notification<T, Contents>, &World) -> VerifyResult;

/// Type of the notification handler ID for verify notifications.
type VerifyHandlerId<T, C> = NotificationHandlerId<T, C, VerifyNotificationHandlers<T, C>>;

// TODO consider removing this duplication

/// The set of verify notification handlers for a single notification type and contents type combination.
#[derive(Resource)]
pub struct VerifyNotificationHandlers<T: NotificationType, C: Send + Sync> {
    /// The ID to be assigned to the next registered handler.
    next_id: NotificationHandlerId<T, C, VerifyNotificationHandlers<T, C>>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<VerifyHandlerId<T, C>, HandleVerifyFn<T, C>>,
}

impl<T: NotificationType + 'static, C: Send + Sync + 'static> VerifyNotificationHandlers<T, C> {
    /// Creates a new, empty set of handlers.
    fn new() -> VerifyNotificationHandlers<T, C> {
        VerifyNotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    /// Adds the provided handler to this set of handlers and returns its assigned ID.
    fn add(&mut self, handle_fn: HandleVerifyFn<T, C>) -> VerifyHandlerId<T, C> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(handler: HandleVerifyFn<T, C>, world: &mut World) -> VerifyHandlerId<T, C> {
        if let Some(mut handlers) = world.get_resource_mut::<VerifyNotificationHandlers<T, C>>() {
            return handlers.add(handler);
        }

        let mut handlers = VerifyNotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: VerifyHandlerId<T, C>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(mut handlers) = world.get_resource_mut::<VerifyNotificationHandlers<T, C>>() {
            handlers.handlers.remove(&id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<VerifyNotificationHandlers<T, C>>();
        }
    }
}
