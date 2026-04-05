use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

use bevy_ecs::prelude::*;

//TODO use bevy observers instead of all this?

/// Trait for types that represent a category of notifications.
///
/// A given type should either implement this or `ReturningNotificationType`, but not both.
pub trait NotificationType: Debug + Send + Sync {}

/// Trait for types that represent a category of notifications that return something.
///
/// A given type should either implement this or `NotificationType`, but not both.
pub trait ReturningNotificationType: Debug + Send + Sync {
    type Return;
}

/// A notification.
#[derive(Debug)]
pub struct Notification<'c, T, Contents> {
    /// The type of the notification.
    pub notification_type: T,
    /// The contents of the notification.
    pub contents: &'c Contents,
}

impl<T: NotificationType + 'static> Notification<'_, T, ()> {
    /// Sends a notification with the provided type and no contents.
    pub fn send_no_contents(notification_type: T, world: &mut World) {
        Notification {
            notification_type,
            contents: &(),
        }
        .send(world)
    }
}

impl<T: ReturningNotificationType<Return = R> + 'static, R: 'static> Notification<'_, T, ()> {
    /// Sends a returning notification with the provided type and no contents.
    pub fn send_no_contents_returning(notification_type: T, world: &mut World) -> Vec<R> {
        Notification {
            notification_type,
            contents: &(),
        }
        .send_returning(world)
    }
}

impl<T: NotificationType + 'static, C: Send + Sync + 'static> Notification<'_, T, C> {
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

impl<T: ReturningNotificationType<Return = R> + 'static, C: Send + Sync + 'static, R: 'static>
    Notification<'_, T, C>
{
    /// Sends this notification to all the handlers registered for it.
    pub fn send_returning(&self, world: &World) -> Vec<R> {
        let mut returned = Vec::new();
        if let Some(handlers) = world.get_resource::<ReturningNotificationHandlers<T, C, R>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .cloned()
                .collect::<Vec<ReturningHandleFn<T, C, R>>>();

            for handle_fn in handle_fns {
                returned.push(handle_fn(self, world));
            }
        }

        returned
    }
}

/// An identifier for a registered notification handler.
///
/// This is only unique to the notification type + contents + return type combo.
/// For example, the first handler registered for `BeforeActionNotification` and `MoveAction` and the first one registered for
/// `BeforeActionNotification` and `LookAction` will both have the same internal value, just different associated types.
pub struct NotificationHandlerId<T, C: Send + Sync, R> {
    value: u64,
    _t: PhantomData<fn(T)>,
    _c: PhantomData<fn(C)>,
    _r: PhantomData<fn(R)>,
}

// need to manually implement traits due to https://github.com/rust-lang/rust/issues/26925
impl<T: Debug + Send + Sync, C: Send + Sync, R> Clone for NotificationHandlerId<T, C, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Debug + Send + Sync, C: Send + Sync, R> Copy for NotificationHandlerId<T, C, R> {}

impl<T: Debug + Send + Sync, C: Send + Sync, R> PartialEq for NotificationHandlerId<T, C, R> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Debug + Send + Sync, C: Send + Sync, R> Eq for NotificationHandlerId<T, C, R> {}

impl<T: Debug + Send + Sync, C: Send + Sync, R> Hash for NotificationHandlerId<T, C, R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T, C: Send + Sync, R> NotificationHandlerId<T, C, R> {
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

/// Signature of a function to handle notifications that can mutate the world and returns nothing.
type HandleFn<T, Contents> = fn(&Notification<T, Contents>, &mut World);

/// Signature of a function to handle notifications that can't mutate the world and returns something.
type ReturningHandleFn<T, Contents, Return> = fn(&Notification<T, Contents>, &World) -> Return;

/// The set of notification handlers that take a mutable world and return nothing for a single notification type and contents type notification.
#[derive(Resource)]
pub struct NotificationHandlers<T: NotificationType, C: Send + Sync> {
    /// The ID to be assigned to the next registered handler.
    next_id: NotificationHandlerId<T, C, ()>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<NotificationHandlerId<T, C, ()>, HandleFn<T, C>>,
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
    fn add(&mut self, handle_fn: HandleFn<T, C>) -> NotificationHandlerId<T, C, ()> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(
        handler: HandleFn<T, C>,
        world: &mut World,
    ) -> NotificationHandlerId<T, C, ()> {
        if let Some(mut handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            return handlers.add(handler);
        }

        let mut handlers = NotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    #[expect(unused)]
    pub fn remove_handler(id: NotificationHandlerId<T, C, ()>, world: &mut World) {
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

/// The set of notification handlers that take an immutable world and return something for a single notification type, contents type, and return type combination.
#[derive(Resource)]
pub struct ReturningNotificationHandlers<
    T: ReturningNotificationType<Return = R>,
    C: Send + Sync,
    R,
> {
    /// The ID to be assigned to the next registered handler.
    next_id: NotificationHandlerId<T, C, R>,
    /// The handlers, keyed by their assigned IDs.
    handlers: HashMap<NotificationHandlerId<T, C, R>, ReturningHandleFn<T, C, R>>,
}

impl<T: ReturningNotificationType<Return = R> + 'static, C: Send + Sync + 'static, R: 'static>
    ReturningNotificationHandlers<T, C, R>
{
    /// Creates a new, empty set of handlers.
    fn new() -> ReturningNotificationHandlers<T, C, R> {
        ReturningNotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    /// Adds the provided handler to this set of handlers and returns its assigned ID.
    fn add(&mut self, handle_fn: ReturningHandleFn<T, C, R>) -> NotificationHandlerId<T, C, R> {
        let id = self.next_id;
        self.handlers.insert(id, handle_fn);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler function. Returns an ID that can be used to remove the handler later.
    pub fn add_handler(
        handler: ReturningHandleFn<T, C, R>,
        world: &mut World,
    ) -> NotificationHandlerId<T, C, R> {
        if let Some(mut handlers) =
            world.get_resource_mut::<ReturningNotificationHandlers<T, C, R>>()
        {
            return handlers.add(handler);
        }

        let mut handlers = ReturningNotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    #[expect(unused)]
    pub fn remove_handler(id: NotificationHandlerId<T, C, R>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(mut handlers) =
            world.get_resource_mut::<ReturningNotificationHandlers<T, C, R>>()
        {
            handlers.handlers.remove(&id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<ReturningNotificationHandlers<T, C, R>>();
        }
    }
}
