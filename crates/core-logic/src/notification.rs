use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

use bevy_ecs::prelude::*;

pub trait NotificationType: Debug + Send + Sync {}

#[derive(Debug)]
pub struct BeforeActionNotification {
    pub performing_entity: Entity,
}

impl NotificationType for BeforeActionNotification {}

#[derive(Debug)]
pub struct AfterActionNotification {
    pub performing_entity: Entity,
}

impl NotificationType for AfterActionNotification {}

#[derive(Debug)]
pub struct Notification<'c, T: NotificationType, Contents> {
    pub notification_type: T,
    pub contents: &'c Contents,
}

impl<'c, T: NotificationType + 'static, C: Send + Sync + 'static> Notification<'c, T, C> {
    /// Sends this notification to all the handlers registered for it.
    pub fn send(&self, world: &mut World) {
        if let Some(handlers) = world.get_resource::<NotificationHandlers<T, C>>() {
            let handle_fns = handlers
                .handlers
                .values()
                .map(|h| h.handle_fn)
                .collect::<Vec<HandleFn<T, C>>>();

            for handle_fn in handle_fns {
                handle_fn(self, world);
            }
        }
    }
}

type HandleFn<T, Contents> = fn(&Notification<T, Contents>, &mut World);

pub struct NotificationHandler<T: NotificationType, C: Send + Sync> {
    pub handle_fn: HandleFn<T, C>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct NotificationHandlerId<T: NotificationType, C: Send + Sync> {
    value: u64,
    _t: PhantomData<fn(T)>,
    _c: PhantomData<fn(C)>,
}

impl<T: NotificationType, C: Send + Sync> NotificationHandlerId<T, C> {
    fn new() -> NotificationHandlerId<T, C> {
        NotificationHandlerId {
            value: 0,
            _t: PhantomData,
            _c: PhantomData,
        }
    }

    fn next(self) -> NotificationHandlerId<T, C> {
        self.value += 1;
        self
    }
}

#[derive(Resource)]
pub struct NotificationHandlers<T: NotificationType, C: Send + Sync> {
    next_id: NotificationHandlerId<T, C>,
    handlers: HashMap<NotificationHandlerId<T, C>, NotificationHandler<T, C>>,
}

impl<T: NotificationType, C: Send + Sync> NotificationHandlers<T, C> {
    fn new() -> NotificationHandlers<T, C> {
        NotificationHandlers {
            next_id: NotificationHandlerId::new(),
            handlers: HashMap::new(),
        }
    }

    fn add(&mut self, handler: NotificationHandler<T, C>) -> NotificationHandlerId<T, C> {
        let id = self.next_id;
        self.handlers.insert(id, handler);
        self.next_id = self.next_id.next();

        id
    }

    /// Registers the provided handler.
    pub fn add_handler(
        handler: NotificationHandler<T, C>,
        world: &mut World,
    ) -> NotificationHandlerId<T, C> {
        if let Some(handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            return handlers.add(handler);
        }

        let handlers = NotificationHandlers::new();
        let id = handlers.add(handler);
        world.insert_resource(handlers);

        id
    }

    /// Removes the handler with the provided ID.
    pub fn remove_handler(id: NotificationHandlerId<T, C>, world: &mut World) {
        let mut remove_resource = false;
        if let Some(handlers) = world.get_resource_mut::<NotificationHandlers<T, C>>() {
            handlers.remove(id);

            if handlers.handlers.is_empty() {
                remove_resource = true;
            }
        }

        if remove_resource {
            world.remove_resource::<NotificationHandlers<T, C>>();
        }
    }
}
