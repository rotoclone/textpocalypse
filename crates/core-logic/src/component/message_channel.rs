use bevy_ecs::prelude::*;
use flume::Sender;

use crate::{GameMessage, Time};

/// A channel for sending messages to an entity.
#[derive(Component)]
pub struct MessageChannel {
    /// The sender to use to send messages to the entity.
    pub sender: Sender<(GameMessage, Time)>,
}
