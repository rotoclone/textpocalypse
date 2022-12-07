use std::fmt::Debug;

use bevy_ecs::prelude::*;

pub trait NotificationType: Debug {}

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
pub struct Notification<'c, T: NotificationType, C> {
    pub notification_type: T,
    pub contents: &'c C,
}

type HandleFn<T, C> = fn(&Notification<T, C>, &mut World);

pub struct NotificationHandler<T: NotificationType, C> {
    pub handle_fn: HandleFn<T, C>,
}

#[derive(Resource)]
pub struct NotificationHandlers<T: NotificationType, C> {
    handlers: Vec<NotificationHandler<T, C>>,
}

impl<T: NotificationType, C> NotificationHandlers<T, C> {
    pub fn new() -> NotificationHandlers<T, C> {
        NotificationHandlers {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: NotificationHandler<T, C>) {
        self.handlers.push(handler);
    }

    pub fn remove_handler(&mut self, handler: NotificationHandler<T, C>) {
        //TODO self.handlers.retain(|h| h != handler)
    }
}

impl<'c, T: NotificationType + 'static, C: 'static> Notification<'c, T, C> {
    /// Sends this notification to all the handlers registered for it.
    pub fn send(&self, world: &mut World) {
        if let Some(handlers) = world.get_resource::<NotificationHandlers<T, C>>() {
            let handle_fns = handlers
                .handlers
                .iter()
                .map(|h| h.handle_fn)
                .collect::<Vec<HandleFn<T, C>>>();

            for handle_fn in handle_fns {
                handle_fn(self, world);
            }
        }
    }
}
