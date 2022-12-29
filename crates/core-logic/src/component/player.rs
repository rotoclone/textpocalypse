use bevy_ecs::prelude::*;
use flume::Sender;

use crate::{GameMessage, Time};

/// A unique identifier for a player.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerId(pub u32);

impl PlayerId {
    /// Increments this player ID.
    pub fn increment(self) -> PlayerId {
        PlayerId(self.0 + 1)
    }
}

#[derive(Component)]
pub struct Player {
    /// The unique ID of the player.
    pub id: PlayerId,
    /// The sender to use to send messages to the entity.
    pub sender: Sender<(GameMessage, Time)>,
}
