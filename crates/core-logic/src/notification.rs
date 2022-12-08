use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

use bevy_ecs::prelude::*;

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
}

/// An identifier for a registered notification handler.
///
/// This is only unique to the notification type + contents type combo.
/// For example, the first handler registered for `BeforeActionNotification` and `MoveAction` and the first one registered for
/// `BeforeActionNotification` and `LookAction` will both have the same internal value, just different associated types.
pub struct NotificationHandlerId<T: NotificationType, C: Send + Sync> {
    value: u64,
    _t: PhantomData<fn(T)>,
    _c: PhantomData<fn(C)>,
}

// need to manually implement traits due to https://github.com/rust-lang/rust/issues/26925
impl<T: NotificationType, C: Send + Sync> Clone for NotificationHandlerId<T, C> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            _t: PhantomData,
            _c: PhantomData,
        }
    }
}

impl<T: NotificationType, C: Send + Sync> Copy for NotificationHandlerId<T, C> {}

impl<T: NotificationType, C: Send + Sync> PartialEq for NotificationHandlerId<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: NotificationType, C: Send + Sync> Eq for NotificationHandlerId<T, C> {}

impl<T: NotificationType, C: Send + Sync> Hash for NotificationHandlerId<T, C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: NotificationType, C: Send + Sync> NotificationHandlerId<T, C> {
    /// Creates a new notification handler ID with the minimum starting value.
    fn new() -> NotificationHandlerId<T, C> {
        NotificationHandlerId {
            value: 0,
            _t: PhantomData,
            _c: PhantomData,
        }
    }

    /// Increments this notification handler ID's value.
    fn next(mut self) -> NotificationHandlerId<T, C> {
        self.value += 1;
        self
    }
}

/// Signature of a funtion to handle notifications.
type HandleFn<T, Contents> = fn(&Notification<T, Contents>, &mut World);

/// The set of notification handlers for a single notification type and contents type combination.
#[derive(Resource)]
pub struct NotificationHandlers<T: NotificationType, C: Send + Sync> {
    /// The ID to be assigned to the next registered handler.
    next_id: NotificationHandlerId<T, C>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<NotificationHandlerId<T, C>, HandleFn<T, C>>,
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
    fn add(&mut self, handle_fn: HandleFn<T, C>) -> NotificationHandlerId<T, C> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(handler: HandleFn<T, C>, world: &mut World) -> NotificationHandlerId<T, C> {
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            return handlers.add(handler);
        }

        let mut handlers = NotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: NotificationHandlerId<T, C>, world: &mut World) {
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
